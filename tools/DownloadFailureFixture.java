import java.nio.file.*;
import org.dcm4che3.data.*;
import org.dcm4che3.io.DicomOutputStream;

/** Test-only multiframe noise; streamed to disk without a full-volume allocation. */
public class DownloadFailureFixture {
    public static void main(String[] args) throws Exception {
        int frames=Integer.parseInt(args[1]), side=512;
        var a=new Attributes();
        a.setString(Tag.SOPClassUID,VR.UI,"1.2.840.10008.5.1.4.1.1.7.2");
        a.setString(Tag.SOPInstanceUID,VR.UI,"1.2.3.4.5");
        a.setString(Tag.StudyInstanceUID,VR.UI,"1.2.3");a.setString(Tag.SeriesInstanceUID,VR.UI,"1.2.3.4");
        a.setString(Tag.PatientID,VR.LO,"SYNTHETIC-FAILURE-TEST");
        a.setInt(Tag.Rows,VR.US,side);a.setInt(Tag.Columns,VR.US,side);
        a.setInt(Tag.SamplesPerPixel,VR.US,1);a.setString(Tag.PhotometricInterpretation,VR.CS,"MONOCHROME2");
        a.setInt(Tag.BitsAllocated,VR.US,8);a.setInt(Tag.BitsStored,VR.US,8);
        a.setInt(Tag.HighBit,VR.US,7);a.setInt(Tag.PixelRepresentation,VR.US,0);
        a.setInt(Tag.NumberOfFrames,VR.IS,frames);
        byte[] data=new byte[side*side];int noise=0x12345678;
        try(var out=new DicomOutputStream(Path.of(args[0]).toFile())) {
            out.writeDataset(a.createFileMetaInformation(UID.ExplicitVRLittleEndian),a);
            out.writeHeader(Tag.PixelData,VR.OB,Math.multiplyExact(frames,data.length));
            for(int frame=0;frame<frames;frame++) {
                for(int i=0;i<data.length;i++) {noise^=noise<<13;noise^=noise>>>17;noise^=noise<<5;data[i]=(byte)noise;}
                out.write(data);
            }
        }
        System.out.println("Synthetic multiframe DICOM generated: "+frames+" frames, 512x512");
    }
}
