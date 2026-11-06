// Ported from Java revision 2373896f; DICOM behavior retained, no Spring/business persistence.
package vn.saolasoft.pacs.dicom.keyimage;

import jakarta.json.Json;
import jakarta.json.stream.JsonGenerator;
import org.apache.commons.io.IOUtils;
import org.dcm4che3.data.Attributes;
import org.dcm4che3.data.Tag;
import org.dcm4che3.data.UID;
import org.dcm4che3.imageio.codec.Transcoder;
import org.dcm4che3.io.DicomInputStream;
import org.dcm4che3.io.DicomOutputStream;
import org.dcm4che3.json.JSONWriter;
import org.dcm4che3.util.TagUtils;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.*;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.stream.Stream;

public class FileStorageServiceImpl {

    private final Logger logger = LoggerFactory.getLogger(this.getClass());

    private final String METADATA_FILE_NAME = "metadata.json";

    private final String DICOM_EXTENSION = ".dcm";

    private String keyImageDirectory;
    public FileStorageServiceImpl(String root) { keyImageDirectory=root; }

    private Path contained(String series,String file) throws IOException {
        Path root=Path.of(keyImageDirectory).toAbsolutePath().normalize();
        Path path=root.resolve(series).resolve(file).normalize();
        if(!path.startsWith(root) || path.equals(root))throw new IOException("Key image file outside storage");
        return path;
    }
    public String saveKeyImage(String transferSyntax, Attributes attrs) throws IOException {
        String seriesInstanceUID = attrs.getString(Tag.SeriesInstanceUID);
        String sopInstanceUID = attrs.getString(Tag.SOPInstanceUID);

        contained(seriesInstanceUID,sopInstanceUID+DICOM_EXTENSION);
        String folderDir = FileUtils.createStoredDir(keyImageDirectory, seriesInstanceUID);
        String keyImage = folderDir + "/" + sopInstanceUID + DICOM_EXTENSION;

        // Write key key image to dicom in transferSyntax = UID.JPEGBaseline8Bit
        try (DicomOutputStream dos = new DicomOutputStream(new FileOutputStream(keyImage), UID.ExplicitVRLittleEndian)) {
            Attributes fmi = attrs.createFileMetaInformation(transferSyntax);
            dos.writeDataset(fmi, attrs);
        }
        return keyImage;
    }

    public InputStream getSeriesMetadata(String seriesInstanceUID) {
        try {
            String path = contained(seriesInstanceUID,METADATA_FILE_NAME).toString();
            File file = new File(path);
            if (!file.exists()) {
                logger.error("[SeriesMetadata] File not found-{}", path);
                return null;
            }
            return new FileInputStream(file);
        } catch (IOException e) {
            logger.error("[SeriesMetadata] File not found");
            return null;
        }
    }

    public InputStream getInstanceFrame(String seriesInstanceUID, String sopInstanceUID) {
        try {
            String path = contained(seriesInstanceUID,sopInstanceUID+DICOM_EXTENSION).toString();
            File file = new File(path);
            if (!file.exists()) {
                logger.error("[InstanceFrame] File not found-{}", path);
                return null;
            }
            // Read frame key image in transfer syntax = UID.JPEGBaseline8Bit
            return writeCompressedSingleFrame(new FileInputStream(file), UID.JPEGBaseline8Bit);
        } catch (Exception e) {
            logger.error("[InstanceFrame] Get instance frame error", e);
            return null;
        }

    }

    private InputStream writeCompressedSingleFrame(InputStream inputStream, String transferSyntax) throws IOException {
        InputStream s1 = new ByteArrayInputStream(("--" + FileConstant.BOUNDARY + "\r\n").getBytes());
        InputStream s2 = new ByteArrayInputStream(("Content-Type: image/jpeg; transfer-syntax=" + transferSyntax + "\r\n\r\n").getBytes());
        CompressedSingleFrameInputStream s3 = new CompressedSingleFrameInputStream(inputStream);
        InputStream s4 = new ByteArrayInputStream(("\r\n--" + FileConstant.BOUNDARY + "--").getBytes());
        List<InputStream> streams = Arrays.asList(s1, s2, s3, s4);
        return new SequenceInputStream(Collections.enumeration(streams));
    }

    public void saveSeriesMetadata(List<Attributes> attrs) throws IOException {
        Attributes firstAttributes = attrs.get(0);
        String seriesInstanceUID = firstAttributes.getString(Tag.SeriesInstanceUID);

        String metadataFile = keyImageDirectory + "/" + seriesInstanceUID + "/" + METADATA_FILE_NAME;

        List<Attributes> combineAttributes = new ArrayList<>(attrs);
        if (Files.exists(Path.of(metadataFile))) {
            try (InputStream in = new FileInputStream(metadataFile)) { combineAttributes.addAll(DicomWebUtil.parseJSON(in)); }
        }

        try (JsonGenerator gen = Json.createGenerator(new FileOutputStream(metadataFile))) {
        JSONWriter writer = new JSONWriter(gen);
        gen.writeStartArray();

        for (Attributes attr : combineAttributes) {
            writer.write(attr);
        }
        gen.writeEnd();
        gen.flush();
        }
    }

}
