package vn.saolasoft.pacs.dicom.print;

import org.apache.commons.imaging.Imaging;
import org.dcm4che3.data.Attributes;
import org.dcm4che3.data.Sequence;
import org.dcm4che3.data.Tag;
import org.dcm4che3.data.UID;
import org.dcm4che3.data.VR;
import org.dcm4che3.net.ApplicationEntity;
import org.dcm4che3.net.Association;
import org.dcm4che3.net.Commands;
import org.dcm4che3.net.Connection;
import org.dcm4che3.net.Device;
import org.dcm4che3.net.Dimse;
import org.dcm4che3.net.DimseRQHandler;
import org.dcm4che3.net.DimseRSP;
import org.dcm4che3.net.PDVInputStream;
import org.dcm4che3.net.Status;
import org.dcm4che3.net.TransferCapability;
import org.dcm4che3.net.pdu.AAssociateRQ;
import org.dcm4che3.net.pdu.PresentationContext;
import org.dcm4che3.util.UIDUtils;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.awt.*;
import java.awt.color.ColorSpace;
import java.awt.image.BufferedImage;
import java.awt.image.ColorModel;
import java.awt.image.ComponentColorModel;
import java.awt.image.DataBuffer;
import java.awt.image.DataBufferByte;
import java.awt.image.DataBufferShort;
import java.awt.image.DataBufferUShort;
import java.awt.image.WritableRaster;
import java.io.IOException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;

import java.nio.file.Files;
import java.nio.file.Path;
import jakarta.json.Json;

/** One call owns the complete print transaction and all network resources. */
public final class PrintSupport {
    private final Logger logger = LoggerFactory.getLogger(PrintSupport.class);
    private final int timeoutMillis;
    private boolean actionPending;
    private PrintSupport(int timeoutMillis) { this.timeoutMillis = timeoutMillis; }
    public record PrintOptions(int numOfCopies, String priority, String mediumType, String filmDestination, String filmOrientation, String filmSizeId, String magnificationType, String smoothingType, String trim, String borderDensity, int minDensity, int maxDensity, String imageDisplayFormat, boolean colorPrint) {
        public int getNumOfCopies() { return numOfCopies; }
        public String getPriority() { return priority; }
        public String getMediumType() { return mediumType; }
        public String getFilmDestination() { return filmDestination; }
        public String getFilmOrientation() { return filmOrientation; }
        public String getFilmSizeId() { return filmSizeId; }
        public String getMagnificationType() { return magnificationType; }
        public String getSmoothingType() { return smoothingType; }
        public String getTrim() { return trim; }
        public String getBorderDensity() { return borderDensity; }
        public int getMinDensity() { return minDensity; }
        public int getMaxDensity() { return maxDensity; }
        public String getImageDisplayFormat() { return imageDisplayFormat; }
        public boolean isColorPrint() { return colorPrint; }
    }
    public record PrinterSettings(String host, int port, String aeTitle, String calledAeTitle) {
        public String getHost() { return host; }
        public int getPort() { return port; }
        public String getAeTitle() { return aeTitle; }
        public String getCalledAeTitle() { return calledAeTitle; }
    }

    public static String run(String manifest) throws Exception {
        var path = Path.of(manifest).toRealPath();
        try (var reader = Json.createReader(Files.newBufferedReader(path))) {
            var value = reader.readObject();
            var imagePath = Path.of(value.getString("image")).toRealPath();
            if (!imagePath.getParent().equals(path.getParent())) throw new IOException("Print input sandbox");
            var printer = new PrinterSettings(value.getString("host"), value.getInt("port"),
                    value.getString("aeTitle"), value.getString("calledAeTitle", "VIEWER_AET"));
            var input = value.getJsonObject("options");
            var options = new PrintOptions(
                input.getInt("numOfCopies", 1),
                input.getString("priority", "LOW"),
                input.getString("mediumType", "BLUE FILM"),
                input.getString("filmDestination", "MAGAZINE"),
                input.getString("filmOrientation", "PORTRAIT"),
                input.getString("filmSizeId", "A4"),
                input.getString("magnificationType", "CUBIC"),
                input.getString("smoothingType", "MEDIUM"),
                input.getString("trim", "NO"),
                input.getString("borderDensity", "BLACK"),
                input.getInt("minDensity", 0),
                input.getInt("maxDensity", 255),
                input.getString("imageDisplayFormat", "STANDARD\\1,1"),
                input.getBoolean("colorPrint", false));
            var transaction = new PrintSupport(value.getInt("timeoutMillis"));
            try {
                var image = Imaging.getBufferedImage(imagePath.toFile());
                if (image == null) return result(false, "Image is null");
                try {
                    transaction.printImage(image, options, printer);
                } finally { image.flush(); }
                return result(true, "Print DICOM image successfully");
            } catch (Exception error) {
                return Json.createObjectBuilder().add("succeed", false)
                    .add("message", "Print DICOM image error: " + error.getMessage())
                    .add("outcomeUnknown", transaction.actionPending).build().toString();
            }
        }
    }
    private static String result(boolean succeed, String message) {
        return Json.createObjectBuilder().add("succeed", succeed).add("message", message).build().toString();
    }
    public void printImage(BufferedImage image, PrintOptions printOptions, PrinterSettings printer) throws Exception {
        Attributes filmSessionAttrs = new Attributes();
        Attributes filmBoxAttrs = new Attributes();
        Attributes imageBoxAttrs = new Attributes();
        Attributes dicomImage = new Attributes();
        final String printManagementSOPClass =
                printOptions.isColorPrint()
                        ? UID.BasicColorPrintManagementMeta
                        : UID.BasicGrayscalePrintManagementMeta;
        final String imageBoxSOPClass =
                printOptions.isColorPrint() ? UID.BasicColorImageBox : UID.BasicGrayscaleImageBox;

        storeRasterInDicom(image, dicomImage, printOptions.isColorPrint());

        String weasisAet = (printer.getCalledAeTitle() != null && !printer.getCalledAeTitle().isEmpty()) ? printer.getCalledAeTitle() : "VIEWER_AET";
        Device device = new Device(weasisAet);
        ApplicationEntity ae = new ApplicationEntity(weasisAet);

        ae.addTransferCapability(new TransferCapability(
                null,
                UID.Printer,                    // 1.2.840.10008.5.1.1.16
                TransferCapability.Role.SCP,            // we accept events → act as SCP
                UID.ImplicitVRLittleEndian
        ));

        ae.setDimseRQHandler(new DimseRQHandler() {
            @Override
            public void onDimseRQ(Association association, PresentationContext pc, Dimse dimse, Attributes rq, PDVInputStream dataStream) throws IOException {
                if (dimse == Dimse.N_EVENT_REPORT_RQ) {
                    Attributes data = null;
                    try {
                        if (dataStream != null && rq.getInt(Tag.CommandDataSetType, 0x0101) != 0x0101) {
                            data = dataStream.readDataset(pc.getTransferSyntax());  // read the dataset (Printer Status + Info)
                        }
                    } catch (Exception e) {
                        logger.warn("[PRINTER] Failed to read N-EVENT-REPORT dataset", e);
                    }

                    String status     = data != null ? data.getString(Tag.PrinterStatus, "UNKNOWN") : "UNKNOWN";
                    String statusInfo = data != null ? data.getString(Tag.PrinterStatusInfo, "") : "";

                    logger.info("[PRINTER] Printer Event Report → Status: {} | Info: {}", status, statusInfo);

                    // ALWAYS answer with success
                    association.writeDimseRSP(pc, Commands.mkNEventReportRSP(rq, Status.Success), null);
                }
            }

            @Override
            public void onClose(Association association) {
            }
        });

        Connection conn = new Connection();
        conn.setConnectTimeout(timeoutMillis);
        conn.setResponseTimeout(timeoutMillis);
        conn.setReleaseTimeout(timeoutMillis);

        ApplicationEntity remoteAE = new ApplicationEntity(printer.getAeTitle());
        Connection remoteConn = new Connection();

        ae.addConnection(conn);
        ae.setAssociationInitiator(true);
        ae.setAETitle(weasisAet);

        remoteConn.setPort(printer.getPort());
        remoteConn.setHostname(printer.getHost());
        remoteConn.setSocketCloseDelay(90);

        remoteAE.setAssociationAcceptor(true);
        remoteAE.addConnection(remoteConn);

        device.addConnection(conn);
        device.addApplicationEntity(ae);
        ae.addConnection(conn);
        device.setExecutor(Executors.newSingleThreadExecutor());
        device.setScheduledExecutor(Executors.newSingleThreadScheduledExecutor());

        filmSessionAttrs.setInt(Tag.NumberOfCopies, VR.IS, printOptions.getNumOfCopies());
        filmSessionAttrs.setString(Tag.PrintPriority, VR.CS, printOptions.getPriority());
        filmSessionAttrs.setString(Tag.MediumType, VR.CS, printOptions.getMediumType());
        filmSessionAttrs.setString(Tag.FilmDestination, VR.CS, printOptions.getFilmDestination());
        filmBoxAttrs.setString(Tag.FilmSizeID, VR.CS, printOptions.getFilmSizeId());
        filmBoxAttrs.setString(Tag.FilmOrientation, VR.CS, printOptions.getFilmOrientation());
        filmBoxAttrs.setString(Tag.MagnificationType, VR.CS, printOptions.getMagnificationType());
        filmBoxAttrs.setString(Tag.SmoothingType, VR.CS, printOptions.getSmoothingType());
        filmBoxAttrs.setString(Tag.Trim, VR.CS, printOptions.getTrim());
        filmBoxAttrs.setString(Tag.BorderDensity, VR.CS, printOptions.getBorderDensity());
        filmBoxAttrs.setInt(Tag.MinDensity, VR.US, printOptions.getMinDensity());
        filmBoxAttrs.setInt(Tag.MaxDensity, VR.US, printOptions.getMaxDensity());
        filmBoxAttrs.setString(Tag.ImageDisplayFormat, VR.ST, printOptions.getImageDisplayFormat());
        imageBoxAttrs.setInt(Tag.ImageBoxPosition, VR.US, 1);

        Sequence seq =
                imageBoxAttrs.ensureSequence(
                        printOptions.isColorPrint()
                                ? Tag.BasicColorImageSequence
                                : Tag.BasicGrayscaleImageSequence,
                        1);
        seq.add(dicomImage);
        final String filmSessionUID = UIDUtils.createUID();
        final String filmBoxUID = UIDUtils.createUID();
        Attributes filmSessionSequenceObject = new Attributes();
        filmSessionSequenceObject.setString(Tag.ReferencedSOPClassUID, VR.UI, UID.BasicFilmSession);
        filmSessionSequenceObject.setString(Tag.ReferencedSOPInstanceUID, VR.UI, filmSessionUID);
        seq = filmBoxAttrs.ensureSequence(Tag.ReferencedFilmSessionSequence, 1);
        seq.add(filmSessionSequenceObject);

        AAssociateRQ rq = new AAssociateRQ();
        rq.addPresentationContext(
                new PresentationContext(1, printManagementSOPClass, UID.ImplicitVRLittleEndian));
        rq.setCallingAET(ae.getAETitle());
        rq.setCalledAET(remoteAE.getAETitle());
        Association as = null;
        try {
            as = ae.connect(remoteConn, rq);
            // See http://dicom.nema.org/medical/dicom/current/output/chtml/part02/sect_E.4.2.html
            // Create a Basic Film Session
            logger.info("[PRINTER] Create a Basic Film Session");
            dimseRSPHandler(
                    as.ncreate(
                            printManagementSOPClass,
                            UID.BasicFilmSession,
                            filmSessionUID,
                            filmSessionAttrs,
                            UID.ImplicitVRLittleEndian));
            logger.info("[PRINTER] Done message with the Film Session");

            logger.info("[PRINTER] Create a Basic Film Box");
            // Create a Basic Film Box. We need to get the Image Box UID from the response
            DimseRSP ncreateFilmBoxRSP =
                    as.ncreate(
                            printManagementSOPClass,
                            UID.BasicFilmBox,
                            filmBoxUID,
                            filmBoxAttrs,
                            UID.ImplicitVRLittleEndian);
            dimseRSPHandler(ncreateFilmBoxRSP);
            logger.info("[PRINTER] Done message with the Film Box");
            ncreateFilmBoxRSP.next();

            Attributes filmBoxDataset = ncreateFilmBoxRSP.getDataset();
            if (filmBoxDataset == null) throw new IOException("Printer N-CREATE-RSP returned no dataset");
            Attributes imageBoxSequence = filmBoxDataset.getNestedDataset(Tag.ReferencedImageBoxSequence);
            // Send N-SET message with the Image Box
            logger.info("[PRINTER] Send N-SET message with the Image Box");
            dimseRSPHandler(
                    as.nset(
                            printManagementSOPClass,
                            imageBoxSOPClass,
                            imageBoxSequence.getString(Tag.ReferencedSOPInstanceUID),
                            imageBoxAttrs,
                            UID.ImplicitVRLittleEndian));
            logger.info("[PRINTER] Done message with the Image Box");

            // Send N-ACTION message with the print action
            logger.info("[PRINTER] Send N-ACTION message with the print action");
            actionPending = true;
            dimseRSPHandler(
                    as.naction(
                            printManagementSOPClass,
                            UID.BasicFilmBox,
                            filmBoxUID,
                            1,
                            null,
                            UID.ImplicitVRLittleEndian));
            logger.info("[PRINTER] Done message with the print action");

            // The print action ends here. This will only delete the Film Box and Film Session

            logger.info("[PRINTER] Send N-DELETE message with the Film Box");
            as.ndelete(printManagementSOPClass, UID.BasicFilmBox, filmBoxUID);
            logger.info("[PRINTER] Done message with the Film Box");

            logger.info("[PRINTER] Send N-DELETE message with the Film Session");
            as.ndelete(printManagementSOPClass, UID.BasicFilmSession, filmSessionUID);
            logger.info("[PRINTER] Done message with the Film Session");

        } finally {
            try {
                if (as != null && as.isReadyForDataTransfer()) {
                    as.waitForOutstandingRSP();
                    as.release();
                }
            } finally {
                if (as != null && as.isReadyForDataTransfer()) as.abort();
                if (device.getExecutor() instanceof ExecutorService executor) executor.shutdownNow();
                if (device.getScheduledExecutor() instanceof ScheduledExecutorService scheduler) scheduler.shutdownNow();
            }
        }
    }

    private void dimseRSPHandler(DimseRSP response) throws IOException, InterruptedException {
        response.next();
        Attributes command = response.getCommand();
        int status = command.getInt(Tag.Status, 0);
        actionPending = false;
        if (status == Status.AttributeValueOutOfRange
                || status == Status.AttributeListError
                || status == 0xB600
                || status == 0xB602
                || status == 0xB604
                || status == 0xB609
                || status == 0xB60A) {
            logger.warn("DICOM Print warning status: {}", Integer.toHexString(status));
        } else if (status != Status.Success) {
            logger.error("DICOM Print error status: {}", Integer.toHexString(status));
            throw new IOException(
                    "Unable to print the image. DICOM response status: " + Integer.toHexString(status));
        }
    }

    private void storeRasterInDicom(
            BufferedImage image, Attributes dcmObj, Boolean printInColor) {
        byte[] bytesOut = null;
        if (dcmObj != null && image != null) {
            dcmObj.setInt(Tag.Columns, VR.US, image.getWidth());
            dcmObj.setInt(Tag.Rows, VR.US, image.getHeight());
            dcmObj.setInt(Tag.PixelRepresentation, VR.US, 0);
            dcmObj.setString(
                    Tag.PhotometricInterpretation, VR.CS, printInColor ? "RGB" : "MONOCHROME2"); // NON-NLS
            dcmObj.setInt(Tag.SamplesPerPixel, VR.US, printInColor ? 3 : 1);
            dcmObj.setInt(Tag.BitsAllocated, VR.US, 8);
            dcmObj.setInt(Tag.BitsStored, VR.US, 8);
            dcmObj.setInt(Tag.HighBit, VR.US, 7);
            // Assumed that the displayed image has always an 1/1 aspect ratio.
            dcmObj.setInt(Tag.PixelAspectRatio, VR.IS, 1, 1);
            // Issue with some PrintSCP servers
            // dcmObj.putString(Tag.TransferSyntaxUID, VR.UI, UID.ImplicitVRLittleEndian);

            DataBuffer dataBuffer;
            if (printInColor) {
                // Must be PixelInterleavedSampleModel
                dcmObj.setInt(Tag.PlanarConfiguration, VR.US, 0);
                dataBuffer = image.getRaster().getDataBuffer();
            } else {
                dataBuffer = convertRGBImageToMonochrome(image).getRaster().getDataBuffer();
            }

            if (dataBuffer instanceof DataBufferByte dataBufferByte) {
                bytesOut = dataBufferByte.getData();
            } else if (dataBuffer instanceof DataBufferShort dataBufferShort) {
                bytesOut = fillShortArray(dataBufferShort.getData());
            } else if (dataBuffer instanceof DataBufferUShort dataBufferUShort) {
                bytesOut = fillShortArray(dataBufferUShort.getData());
            }
            dcmObj.setBytes(Tag.PixelData, VR.OW, bytesOut);
        }
    }

    private byte[] fillShortArray(short[] data) {
        byte[] bytesOut = new byte[data.length * 2];
        for (int i = 0; i < data.length; i++) {
            bytesOut[i * 2] = (byte) (data[i] & 0xFF);
            bytesOut[i * 2 + 1] = (byte) ((data[i] >>> 8) & 0xFF);
        }
        return bytesOut;
    }

    private BufferedImage convertRGBImageToMonochrome(BufferedImage colorImage) {
        if (colorImage.getType() == BufferedImage.TYPE_BYTE_GRAY) {
            return colorImage;
        }
        BufferedImage image =
                new BufferedImage(
                        colorImage.getWidth(), colorImage.getHeight(), BufferedImage.TYPE_BYTE_GRAY);
        Graphics g = image.getGraphics();
        g.drawImage(colorImage, 0, 0, null);
        g.dispose();
        return image;
    }
}
