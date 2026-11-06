// Executes the unchanged Java ingest/validation with synthetic files and an in-memory repository double.
import java.nio.file.*;
import java.io.*;
import java.util.*;
import org.dcm4che3.data.*;
import org.dcm4che3.json.JSONWriter;
import jakarta.json.Json;
import org.springframework.test.util.ReflectionTestUtils;
import static org.mockito.Mockito.*;
import static org.mockito.ArgumentMatchers.*;
import vn.saolasoft.viewer.module.keyimage.jpa.StudyKeyImage;
import vn.saolasoft.viewer.module.keyimage.service.*;
import vn.saolasoft.viewer.module.session.service.*;
import vn.saolasoft.viewer.module.session.service.dto.SessionGetDto;
import vn.saolasoft.viewer.module.stowrs.service.*;
import vn.saolasoft.viewer.module.stowrs.service.dto.StowRSDTO;
import vn.saolasoft.viewer.exception.ViewerException;
import com.fasterxml.jackson.databind.ObjectMapper;
public class Step13Baseline {
 public static void main(String[] args) throws Exception {
  Path root=Path.of(args[1]); Files.createDirectories(root);
  var rows=new ArrayList<Map<String,Object>>();
  for(String kind:List.of("success","missing-boundary","empty","missing-study","maximum","partial-write","multiple-studies","jpeg","raw-pixel","missing-pixel-tag")) {
   Path dir=Files.createTempDirectory(root,"case-");
   var pending=new HashMap<String,StudyKeyImage>();
   var repo=mock(StudyKeyImageService.class);
   when(repo.findById(anyString())).thenAnswer(i->pending.getOrDefault(i.getArgument(0),new StudyKeyImage()));
   when(repo.create(any())).thenAnswer(i->{StudyKeyImage e=i.getArgument(0);pending.put(e.getId(),e);return e;});
   when(repo.update(anyString(),any())).thenAnswer(i->{StudyKeyImage e=i.getArgument(1);pending.put(e.getId(),e);return e;});
   var session=new SessionGetDto();session.setStoreUrl("http://fixture.invalid");session.setStoreAuth("Basic synthetic");
   var sessions=mock(SessionService.class);when(sessions.getSession("s")).thenReturn(session);
   var validation=new ValidationServiceImpl(repo,sessions);ReflectionTestUtils.setField(validation,"maxKeyImagePerStudy",kind.equals("maximum")?0:10);
   Files.createDirectories(dir.resolve("files/1.2.3.4"));
   var storage=new FileStorageServiceImpl();ReflectionTestUtils.setField(storage,"keyImageDirectory",dir.resolve("files").toString());
   var service=new StowRSServiceImpl(validation,storage,dir.resolve("spool").toString());
   var attrs=new Attributes();attrs.setString(Tag.StudyInstanceUID,VR.UI,"1.2.3");attrs.setString(Tag.SeriesInstanceUID,VR.UI,"1.2.3.4");
   attrs.setString(Tag.SOPInstanceUID,VR.UI,"1.2.3.4.5");attrs.setString(Tag.SOPClassUID,VR.UI,UID.SecondaryCaptureImageStorage);
   if(kind.equals("missing-study"))attrs.remove(Tag.StudyInstanceUID);
   byte[] bulk=null;String bulkType="application/octet-stream";
   if(kind.equals("jpeg")) {
    var image=new java.awt.image.BufferedImage(8,8,java.awt.image.BufferedImage.TYPE_BYTE_GRAY);
    var bytes=new ByteArrayOutputStream();javax.imageio.ImageIO.write(image,"jpeg",bytes);bulk=bytes.toByteArray();bulkType="image/jpeg";
   } else if(kind.equals("raw-pixel")||kind.equals("missing-pixel-tag")) {
    bulk=new byte[64];attrs.setInt(Tag.SamplesPerPixel,VR.US,1);attrs.setString(Tag.PhotometricInterpretation,VR.CS,"MONOCHROME2");attrs.setInt(Tag.Rows,VR.US,8);attrs.setInt(Tag.Columns,VR.US,8);attrs.setInt(Tag.BitsAllocated,VR.US,8);attrs.setInt(Tag.BitsStored,VR.US,8);attrs.setInt(Tag.HighBit,VR.US,7);attrs.setInt(Tag.PixelRepresentation,VR.US,0);
    if(kind.equals("missing-pixel-tag"))attrs.remove(Tag.Rows);
   }
   if(bulk!=null) {attrs.setValue(Tag.PixelData,VR.OB,new BulkData(null,"bulk-1",false));attrs.setString(Tag.InstanceCreationDate,VR.DA,"20260101");}
   var objects=new ArrayList<Attributes>();if(!kind.equals("empty"))objects.add(attrs);
   if(kind.equals("partial-write")||kind.equals("multiple-studies")) {
    var second=new Attributes(attrs);second.setString(Tag.SOPInstanceUID,VR.UI,"1.2.3.4.6");
    if(kind.equals("partial-write"))second.remove(Tag.SOPClassUID);else second.setString(Tag.StudyInstanceUID,VR.UI,"9.8.7");objects.add(second);
   }
   var json=new StringWriter();try(var gen=Json.createGenerator(json)){var w=new JSONWriter(gen);gen.writeStartArray();for(var a:objects)w.write(a);gen.writeEnd();}
   String body="--b\r\nContent-Type: application/dicom+json\r\n\r\n"+json+"\r\n--b--\r\n";
   byte[] upload;
   if(bulk==null) upload=body.getBytes(java.nio.charset.StandardCharsets.UTF_8);
   else {var bytes=new ByteArrayOutputStream();bytes.write(("--b\r\nContent-Type: application/dicom+json\r\n\r\n"+json+"\r\n--b\r\nContent-Type: "+bulkType+"\r\nContent-Location: bulk-1\r\n\r\n").getBytes(java.nio.charset.StandardCharsets.UTF_8));bytes.write(bulk);bytes.write("\r\n--b--\r\n".getBytes());upload=bytes.toByteArray();}
   var row=new LinkedHashMap<String,Object>();row.put("case",kind);row.put("requestBase64",Base64.getEncoder().encodeToString(upload));
   try {var response=service.store("s",new StowRSDTO(kind.equals("missing-boundary")?"multipart/related":"multipart/related; boundary=b",new ByteArrayInputStream(upload),"http://localhost/ws/rest/v1/session/s/studies"));row.put("status",response.getStatusCode().value());row.put("body",new String(response.getBody().getInputStream().readAllBytes(),java.nio.charset.StandardCharsets.UTF_8));}
   catch(ViewerException e){row.put("status",e.getAppError().getHttpStatus());row.put("errorCode",e.getAppError().getErrorCode());}
   row.put("pending",pending.values().stream().map(e->Map.of("id",e.getId(),"series",e.getSeriesInstanceUID(),"count",e.getNumOfImage(),"retry",e.getNumOfRetry())).toList());
   try(var files=Files.walk(dir)){row.put("files",files.filter(Files::isRegularFile).map(dir::relativize).map(p->p.toString().replace('\\','/')).sorted().toList());}
   var dicom=new LinkedHashMap<String,String>();
   if(row.get("status").equals(200))try(var paths=Files.walk(dir.resolve("files"))) {
    for(var file:paths.filter(p->p.toString().endsWith(".dcm")).toList())dicom.put(file.getFileName().toString(),Base64.getEncoder().encodeToString(Files.readAllBytes(file)));
   }
   row.put("dicomBase64",dicom);
   rows.add(row);
  }
  new ObjectMapper().writerWithDefaultPrettyPrinter().writeValue(new File(args[0]),rows);
 }
}
