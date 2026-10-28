// Runs the unchanged Java download service with synthetic metadata/transport.
// Native render/transcode are mocked here; codec verification is a separate suite.
import java.nio.file.*;
import java.util.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.zip.*;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.springframework.mock.web.MockHttpServletResponse;
import org.springframework.test.web.servlet.setup.MockMvcBuilders;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.get;
import static org.mockito.Mockito.*;
import static org.mockito.ArgumentMatchers.*;
import vn.saolasoft.viewer.module.file.service.*;
import vn.saolasoft.viewer.module.file.rest.DownloadAPI;
import vn.saolasoft.viewer.module.session.service.*;
import vn.saolasoft.viewer.module.session.service.dto.SessionGetDto;
import vn.saolasoft.viewer.module.util.service.ConfigService;
import vn.saolasoft.viewer.module.wado.feign.*;
import org.orthanc.transcode.*;

public class Step12Baseline {
    static feign.Response response(String body) {
        return feign.Response.builder().status(200).request(feign.Request.create("GET", "/",
            Map.of(), null, StandardCharsets.UTF_8)).body(body.getBytes(StandardCharsets.UTF_8)).build();
    }
    public static void main(String[] args) throws Exception {
        var sessions = mock(SessionService.class);
        var session = new SessionGetDto();
        session.setStoreUrl("http://fixture.invalid/dicom/wado-rs");
        session.setStoreAuth("Basic fixture");
        when(sessions.getSession("s", "1.2.3")).thenReturn(session);
        var rs = mock(WadoRsProxy.class);
        var uri = mock(WadoUriProxy.class);
        var configs = mock(ConfigService.class);
        var transcode = mock(TranscodeService.class);
        when(transcode.transcode(any())).thenReturn(new byte[]{1,2,3});
        String metadata = "[{\"00080018\":{\"vr\":\"UI\",\"Value\":[\"1.2.3.4.5\"]},"
            + "\"00200013\":{\"vr\":\"IS\",\"Value\":[1]},\"00280008\":{\"vr\":\"IS\",\"Value\":[2]}}]";
        when(rs.getSeriesMetadata(any(), any(), any(), any())).thenAnswer(i -> response(metadata));
        when(rs.getInstanceMetadata(any(), any(), any(), any(), any())).thenAnswer(i -> response(metadata));
        when(rs.getSeries(any(), any(), any())).thenAnswer(i -> response(
            "[{\"0020000E\":{\"vr\":\"UI\",\"Value\":[\"1.2.3.4\"]}}]"));
        when(uri.retrieveInstance(any(), any(), any(), any(), any(), any(), any()))
            .thenAnswer(i -> response("fixture"));
        var service = new DownloadServiceImpl(sessions, rs, uri, configs, transcode);
        var mvc = MockMvcBuilders.standaloneSetup(new DownloadAPI(service))
            .setControllerAdvice(new vn.saolasoft.viewer.exception.GlobalExceptionHandler()).build();
        var rows = new ArrayList<Map<String,Object>>();
        String root = "/ws/rest/v1/session/s/download/studies/1.2.3";
        String[] suffixes = {"", "/series/1.2.3.4", "/series/1.2.3.4/instances/1.2.3.4.5"};
        for (String suffix: suffixes) for (String type: List.of("", "unknown", "pdf", "mp4")) {
            // Series MP4 requires real pixels and is covered in codec tests.
            if (type.equals("mp4") && suffix.equals(suffixes[1])) continue;
            var res = mvc.perform(get(root + suffix).param("downloadType",type)).andReturn().getResponse();
            if(res.getStatus()!=415) throw new AssertionError("unsupported " + suffix + type);
            var row = new LinkedHashMap<String,Object>();
            row.put("scope", suffix.isEmpty()?"study":suffix.contains("instances")?"instance":"series");
            row.put("type", type); row.put("status",res.getStatus());
            row.put("errorCode",new ObjectMapper().readTree(res.getContentAsString()).get("errorCode").asText());
            rows.add(row);
        }
        { // Renderer replaced only in the isolated baseline classpath.
            for(String scope: List.of("study","series","instance")) for(String type: List.of("DICOM","JPEG")) {
                var res = new MockHttpServletResponse();
                var stream = switch(scope) {
                    case "study" -> service.downloadStudy("s","1.2.3",DownloadType.parse(type),res);
                    case "series" -> service.downloadSeries("s","1.2.3","1.2.3.4",DownloadType.parse(type),res);
                    default -> service.downloadInstance("s","1.2.3","1.2.3.4","1.2.3.4.5",DownloadType.parse(type),res);
                };
                stream.writeTo(new ByteArrayOutputStream());
                var row = new LinkedHashMap<String,Object>();
                row.put("scope",scope);row.put("type",type);row.put("status",res.getStatus());
                row.put("contentType",res.getContentType());row.put("disposition",res.getHeader("Content-Disposition"));
                var entries = new TreeMap<String,String>();
                if("application/zip".equals(res.getContentType())) {
                    try(var zip = new ZipInputStream(new ByteArrayInputStream(res.getContentAsByteArray()))) {
                        for(ZipEntry e; (e=zip.getNextEntry())!=null;) entries.put(e.getName(),Base64.getEncoder().encodeToString(zip.readAllBytes()));
                    }
                    row.put("entries",entries);
                } else row.put("body",Base64.getEncoder().encodeToString(res.getContentAsByteArray()));
                rows.add(row);
            }
        }
        when(rs.getInstanceMetadata(any(), any(), any(), any(), any())).thenAnswer(i -> response(metadata.replace("[2]","[1]")));
        var single = new MockHttpServletResponse();
        service.downloadInstance("s","1.2.3","1.2.3.4","1.2.3.4.5",DownloadType.JPEG,single).writeTo(new ByteArrayOutputStream());
        rows.add(Map.of("scope","instance","type","JPEG","status",single.getStatus(),"metadataFrames",1,
            "contentType",single.getContentType(),"disposition",single.getHeader("Content-Disposition"),"body",Base64.getEncoder().encodeToString(single.getContentAsByteArray())));
        when(rs.getSeriesMetadata(any(), any(), any(), any())).thenAnswer(i -> response("[]"));
        for(var type:List.of(DownloadType.DICOM,DownloadType.JPEG,DownloadType.MP4)) {
            var empty=new MockHttpServletResponse();
            service.downloadSeries("s","1.2.3","1.2.3.4",type,empty).writeTo(new ByteArrayOutputStream());
            var row=new LinkedHashMap<String,Object>();row.put("scope","series");row.put("type",type.name());row.put("emptySeries",true);
            row.put("status",empty.getStatus());row.put("contentType",empty.getContentType());row.put("disposition",empty.getHeader("Content-Disposition"));
            if(type==DownloadType.MP4) row.put("body",Base64.getEncoder().encodeToString(empty.getContentAsByteArray()));
            else row.put("entries",Map.of());rows.add(row);
        }
        for(String scope: List.of("study","series","instance")) {
            var res = new MockHttpServletResponse();
            var stream = switch(scope) {
                case "study" -> service.downloadStudy("s","outside",DownloadType.DICOM,res);
                case "series" -> service.downloadSeries("s","outside","1.2.3.4",DownloadType.DICOM,res);
                default -> service.downloadInstance("s","outside","1.2.3.4","1.2.3.4.5",DownloadType.DICOM,res);
            };
            if(stream!=null) throw new AssertionError("scope");
            rows.add(Map.of("scope",scope,"type","DICOM","outsideStudy",true,"status",res.getStatus(),"emptyBody",true));
        }
        Files.writeString(Path.of(args[0]),new ObjectMapper().writerWithDefaultPrettyPrinter().writeValueAsString(rows)+"\n");
        if(args.length>1) {
            var images=new ArrayList<java.awt.image.BufferedImage>();
            var reader=javax.imageio.ImageIO.getImageReadersByFormatName("DICOM").next();
            Path directory=Path.of(args[1]);
            try(var input=javax.imageio.ImageIO.createImageInputStream(directory.resolve("input.dcm").toFile())) {
                reader.setInput(input);
                images.add(reader.read(0,reader.getDefaultReadParam()));images.add(reader.read(1,reader.getDefaultReadParam()));
                try(var output=Files.newOutputStream(directory.resolve("baseline-java.mp4"))) { service.convertJpegToMp4(output,images,1); }
            } finally {reader.dispose();}
            System.out.println("PASS original Java MP4 export with synthetic two-frame DICOM");
        }
        System.out.println("PASS Step12 Java contract: "+rows.size()+" cases; codec calls mocked");
    }
}
