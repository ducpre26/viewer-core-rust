// Ported from Java revision 2373896f; DICOM behavior retained, no Spring/business persistence.
package vn.saolasoft.pacs.dicom.keyimage;

import jakarta.ws.rs.core.MediaType;
import org.dcm4che3.data.*;
import org.dcm4che3.imageio.codec.jpeg.JPEGParser;
import org.dcm4che3.imageio.codec.mp4.MP4FileType;
import org.dcm4che3.imageio.codec.mp4.MP4Parser;
import org.dcm4che3.imageio.codec.mpeg.MPEG2Parser;
import org.dcm4che3.ws.rs.MediaTypes;

import java.io.BufferedReader;
import java.io.File;
import java.io.IOException;
import java.io.InputStreamReader;
import java.nio.channels.SeekableByteChannel;

public enum CompressedPixelData {
    JPEG {
        String supplementImagePixelModule(SeekableByteChannel channel, Attributes attrs, boolean fragmented)
                throws IOException {
            JPEGParser jpegParser = new JPEGParser(channel);
            jpegParser.getAttributes(attrs);
            adjustBulkdata(attrs, jpegParser);

            /* Allows retired
            Store received JPEG Full Progression, Non-Hierarchical JPEG images in DICOM images with corresponding (retired)
            Transfer Syntax UID 1.2.840.10008.1.2.4.55.
            Otherwise set 1.2.840.10008.1.2.4.50 (= JPEG Baseline) or 1.2.840.10008.1.2.4.51 (= JPEG Extended)
            as Transfer Syntax UID of the stored DICOM image, without transcoding to JPEG Baseline or JPEG Extended,
            but including the JPEG image as received. May be overwritten by configured value for particular Archive Network AEs.
             */
            // TODO: Hardcode allowRetired = false
            return adjustJPEGTransferSyntax(jpegParser.getTransferSyntaxUID(fragmented),
                    false, attrs);
        }
    },
    MPEG {
        String supplementImagePixelModule(SeekableByteChannel channel, Attributes attrs, boolean fragmented)
                throws IOException {
            MPEG2Parser mpeg2Parser = new MPEG2Parser(channel);
            mpeg2Parser.getAttributes(attrs);
            return mpeg2Parser.getTransferSyntaxUID(fragmented);
        }
    },
    MP4 {
        String supplementImagePixelModule(SeekableByteChannel channel, Attributes attrs, boolean fragmented)
                throws IOException {
            MP4Parser mp4Parser = new MP4Parser(channel);
            mp4Parser.getAttributes(attrs);
            adjustBulkdata(attrs, mp4Parser);
            return mp4Parser.getTransferSyntaxUID(fragmented);
        }
    };

    abstract String supplementImagePixelModule(SeekableByteChannel channel, Attributes attrs, boolean fragmented)
            throws IOException;

    static CompressedPixelData valueOf(MediaType mediaType) {
        return MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.IMAGE_JPEG_TYPE)
                || MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.IMAGE_JP2_TYPE)
                || MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.IMAGE_J2C_TYPE)
                || MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.IMAGE_JPH_TYPE)
                || MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.IMAGE_JPHC_TYPE) ? JPEG
                : MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.VIDEO_MPEG_TYPE) ? MPEG
                : MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.VIDEO_MP4_TYPE)
                || MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.VIDEO_QUICKTIME_TYPE) ? MP4
                : null;
    }

    private static String adjustJPEGTransferSyntax(String tsuid, boolean allowRetired, Attributes attrs) {
        return allowRetired || !tsuid.equals(UID.JPEGFullProgressionNonHierarchical1012)
                ? tsuid
                : attrs.getInt(Tag.BitsAllocated, 8) == 8
                ? UID.JPEGBaseline8Bit
                : UID.JPEGExtended12Bit;
    }

    private static void adjustBulkdata(Attributes attrs, JPEGParser parser) {
        if (parser.getCodeStreamPosition() > 0) {
            excludeJP2FileFormatHeader(parser, attrs);
            return;
        }

        // TODO: Hardcode && false
        if (parser.getPositionAfterAPPSegments() != -1 && false)
            excludeAppMarkers(attrs, parser);
    }

    private static void adjustBulkdata(Attributes attrs, MP4Parser parser) throws IOException {
        // TODO: Hardcode && true
        if (parser.getMP4FileType() != null
                && parser.getMP4FileType().majorBrand() == MP4FileType.qt
                && true) {
            Fragments fragments = (Fragments) attrs.getValue(Tag.PixelData);
            BulkData bulkData = (BulkData) fragments.get(1);
            File mpFile = qt2MP4(bulkData.getFile());
            bulkData.setURI(mpFile.toURI().toString());
            bulkData.setLength(mpFile.length());
        }
    }

    private static File qt2MP4(File qtFile) throws IOException {
        File mp4File = new File(qtFile.getParent(), qtFile.getName() + ".mp4");
        Process process = new ProcessBuilder(
                "ffmpeg", "-i", qtFile.toString(), "-c", "copy", mp4File.toString())
                .redirectErrorStream(true)
                .start();
        String line;
        BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()));
        while ((line = reader.readLine()) != null) {
            // Drain converter output without logging filenames or patient data.
        }
        int exitValue = exitValueOf(process);
        if (exitValue != 0) {
            throw new IOException("Failed to convert Quicktime to MP4 container - " + exitValue);
        }
        return mp4File;
    }

    private static int exitValueOf(Process process) {
        try {
            return process.waitFor();
        } catch (InterruptedException e) {
            return 999;
        }
    }

    private static void excludeJP2FileFormatHeader(JPEGParser parser, Attributes attrs) {
        BulkData bulkData = (BulkData) ((Fragments) attrs.getValue(Tag.PixelData)).get(1);
        bulkData.setLength(bulkData.length() - parser.getCodeStreamPosition());
        bulkData.setOffset(parser.getCodeStreamPosition());
    }

    private static void excludeAppMarkers(Attributes attrs, JPEGParser parser) {
        Fragments fragments = (Fragments) attrs.getValue(Tag.PixelData);
        BulkData bulkData = (BulkData) fragments.get(1);
        fragments.set(1, new BulkDataWithPrefix(
                bulkData.uriWithoutOffsetAndLength(),
                parser.getPositionAfterAPPSegments(),
                (int) (bulkData.length() - parser.getPositionAfterAPPSegments()),
                false,
                (byte) -1, (byte) org.dcm4che3.imageio.codec.jpeg.JPEG.SOI));
    }
}
