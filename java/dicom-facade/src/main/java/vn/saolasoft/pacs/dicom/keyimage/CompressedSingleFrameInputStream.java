// Ported from Java revision 2373896f; DICOM behavior retained, no Spring/business persistence.
package vn.saolasoft.pacs.dicom.keyimage;

import org.dcm4che3.data.Tag;
import org.dcm4che3.io.DicomInputStream;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;

public class CompressedSingleFrameInputStream extends DicomInputStream {

    private final Logger logger = LoggerFactory.getLogger(this.getClass());

    public CompressedSingleFrameInputStream(File file) throws IOException {
        super(file);
        readDataset(-1, Tag.PixelData);
        if (tag() != Tag.PixelData || length() != -1 || !readItemHeader())
            throw new IOException("No or incorrect encapsulated compressed pixel data in requested object");
        skipFully(length());
        readItemHeader();
    }

    public CompressedSingleFrameInputStream(InputStream inputStream) throws IOException {
        super(inputStream);
        readDataset(-1, Tag.PixelData);
        if (tag() != Tag.PixelData || length() != -1 || !readItemHeader())
            throw new IOException("No or incorrect encapsulated compressed pixel data in requested object");
        skipFully(length());
        readItemHeader();
    }


}
