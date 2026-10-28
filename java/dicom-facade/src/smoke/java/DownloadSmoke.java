import java.nio.file.*;
import java.util.*;
import java.io.*;
import org.dcm4che3.data.*;
import org.dcm4che3.io.*;
import org.bytedeco.javacv.FFmpegFrameGrabber;
import vn.saolasoft.pacs.dicom.DicomFacade;

/** Synthetic pixels only. Codec tests intentionally separate from contract stubs. */
public class DownloadSmoke {
    static void dicom(Path path) throws Exception {
        var a=new Attributes();
        a.setString(Tag.SOPClassUID,VR.UI,"1.2.840.10008.5.1.4.1.1.7.2");
        a.setString(Tag.SOPInstanceUID,VR.UI,"1.2.3.4.5");
        a.setString(Tag.PatientID,VR.LO,"SYNTHETIC");
        a.setInt(Tag.Rows,VR.US,16);a.setInt(Tag.Columns,VR.US,16);
        a.setInt(Tag.SamplesPerPixel,VR.US,1);a.setString(Tag.PhotometricInterpretation,VR.CS,"MONOCHROME2");
        a.setInt(Tag.BitsAllocated,VR.US,8);a.setInt(Tag.BitsStored,VR.US,8);
        a.setInt(Tag.HighBit,VR.US,7);a.setInt(Tag.PixelRepresentation,VR.US,0);
        a.setInt(Tag.NumberOfFrames,VR.IS,2);
        byte[] pixels=new byte[512];for(int i=0;i<pixels.length;i++) pixels[i]=(byte)i;
        a.setBytes(Tag.PixelData,VR.OB,pixels);
        try(var out=new DicomOutputStream(path.toFile())) { out.writeDataset(a.createFileMetaInformation(UID.ExplicitVRLittleEndian),a); }
    }
    public static void main(String[] args) throws Exception {
        Path root=Path.of(args[0]).toRealPath();
        Path source=root.resolve("input.dcm");dicom(source);
        String metadata="[{\"00080018\":{\"vr\":\"UI\",\"Value\":[\"1.2.3.4.5\"]},\"00280008\":{\"vr\":\"IS\",\"Value\":[2]}}]";
        String mapped=DicomFacade.downloadMetadata(metadata,"","1",10);
        if(!mapped.contains("undefined-undefined/undefined/1-undefined/1.2.3.4.5") || !mapped.contains("\"frames\":2")) throw new AssertionError(mapped);
        if(!DicomFacade.downloadMetadata("bad","","1",10).equals("[]")) throw new AssertionError("malformed metadata");
        if(!DicomFacade.downloadMetadata(metadata,"{not-a-tag}","1",10).contains("undefined-undefined")) throw new AssertionError("pattern fallback");
        if(!DicomFacade.downloadMetadata(metadata,"custom/{00080018}","1",10).contains("custom/1.2.3.4.5")) throw new AssertionError("custom pattern");
        try { DicomFacade.downloadMetadata(metadata,"","1",0);throw new AssertionError("metadata limit"); }
        catch(IOException expected) {}
        Files.writeString(root.resolve("inputs.txt"),source+"\n");
        DicomFacade.encodeVideo(root.resolve("inputs.txt").toString(),root.resolve("output.mp4").toString(),1,1024*1024,1024*1024,4096,10);
        int count=0;
        try(var grabber=new FFmpegFrameGrabber(root.resolve("output.mp4").toFile())) {
            grabber.start();
            if(grabber.getImageWidth()!=16 || grabber.getImageHeight()!=16) throw new AssertionError("video dimensions");
            while(grabber.grabImage()!=null) count++;
        }
        if(count!=2) throw new AssertionError("frame count "+count);
        try { DicomFacade.encodeVideo(root.resolve("inputs.txt").toString(),root.resolve("limited.mp4").toString(),1,1024*1024,32,4096,10);throw new AssertionError("video output limit"); }
        catch(Exception expected) {}
        if(Files.exists(root.resolve("limited.mp4")) && Files.size(root.resolve("limited.mp4"))>32) throw new AssertionError("output exceeded limit");
        try { DicomFacade.encodeVideo(root.resolve("inputs.txt").toString(),root.resolve("pixel-limit.mp4").toString(),1,1024*1024,1024*1024,1,10);throw new AssertionError("pixel limit"); }
        catch(IOException expected) {}
        System.out.println("PASS metadata, limits and Java binding MP4 encode/decode; frames="+count);
    }
}
