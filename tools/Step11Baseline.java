// Standalone baseline against the unchanged Java classes and their test classpath.
// Real controller/service/Feign contract; synthetic store transport and session data.
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.net.URI;
import java.util.*;
import java.util.concurrent.atomic.*;
import feign.*;
import org.springframework.cloud.openfeign.support.SpringMvcContract;
import org.springframework.test.web.servlet.setup.MockMvcBuilders;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.get;
import static org.mockito.Mockito.*;
import static org.mockito.ArgumentMatchers.*;
import vn.saolasoft.viewer.module.session.service.*;
import vn.saolasoft.viewer.module.session.service.dto.SessionGetDto;
import vn.saolasoft.viewer.module.wado.service.*;
import vn.saolasoft.viewer.module.wado.feign.*;
import vn.saolasoft.viewer.module.wado.rest.WadoRsAPI;
import vn.saolasoft.viewer.module.keyimage.service.*;

public class Step11Baseline {
    public static void main(String[] args) throws Exception {
        var sessions = mock(SessionService.class);
        var session = new SessionGetDto();
        session.setStoreUrl("http://fixture.invalid/dicom/wado-rs");
        session.setStoreAuth("Basic fixture-secret");
        when(sessions.getSession("s", "1.2.3")).thenReturn(session);
        var captured = new AtomicReference<Request>();
        var status = new AtomicInteger(200);
        var includeMedia = new AtomicBoolean(true);
        byte[] payload = new byte[]{0, 1, 2, 13, 10, (byte)255, 42};
        String media = "multipart/related; type=\"application/dicom\"; transfer-syntax=1.2.840.10008.1.2.1; boundary=fixture";
        Client transport = (request, options) -> {
            captured.set(request);
            var headers = new HashMap<String,Collection<String>>(Map.of("Set-Cookie", List.of("secret=fixture"),
                    "Access-Control-Allow-Origin", List.of("https://untrusted.invalid"),
                    "Content-Length", List.of(Integer.toString(payload.length))));
            if(includeMedia.get()) headers.put("cOnTeNt-TyPe",List.of(media));
            return Response.builder().request(request).status(status.get()).reason("Fixture").headers(headers)
                .body(status.get() == 204 ? new byte[0] : payload).build();
        };
        var rs = Feign.builder().contract(new SpringMvcContract()).client(transport).retryer(Retryer.NEVER_RETRY)
            .target(WadoRsProxy.class, "http://fixture.invalid");
        var uri = Feign.builder().contract(new SpringMvcContract()).client(transport).retryer(Retryer.NEVER_RETRY)
            .target(WadoUriProxy.class, "http://fixture.invalid");
        var service = new WadoRsServiceImpl(sessions, rs, uri, mock(StudyKeyImageService.class), mock(FileStorageService.class));
        var mvc = MockMvcBuilders.standaloneSetup(new WadoRsAPI(service))
            .setControllerAdvice(new vn.saolasoft.viewer.exception.GlobalExceptionHandler()).build();
        var output = new ArrayList<Map<String,Object>>();
        String root = "/ws/rest/v1/session/s/wado-rs/studies/1.2.3";
        String[] suffixes = {"", "/thumbnail", "/metadata", "/series", "/series/4.5", "/series/4.5/thumbnail",
            "/series/4.5/metadata", "/series/4.5/instances", "/series/4.5/instances/6.7",
            "/series/4.5/instances/6.7/thumbnail", "/series/4.5/instances/6.7/metadata",
            "/series/4.5/instances/6.7/frames/1,2", "/series/4.5/instances/6.7/frames/1,2/thumbnail",
            "/series/4.5/instances/6.7/bulk/7FE00010"};
        for (String suffix : suffixes) {
            String path = root + suffix;
            var result = mvc.perform(get(URI.create(path + "?quality=90&label=a%20b&quality=10&ignored=yes"))
                .header("Accept", "multipart/related; transfer-syntax=*"))
                .andReturn().getResponse();
            if (result.getStatus() != 200 || !Arrays.equals(result.getContentAsByteArray(), payload))
                throw new AssertionError("route " + suffix);
            output.add(record("route-"+suffix, path, "quality=90&label=a%20b&quality=10&ignored=yes", result, captured.get()));
        }
        for (String query : List.of("studyUID=1.2.3&seriesUID=4.5&objectUID=6.7",
            "studyUID=1.2.3&seriesUID=4.5&objectUID=6.7&requestType=&contentType=image%2Fjpeg&ignored=yes",
            "studyUID=1.2.3&seriesUID=4.5&objectUID=6.7&requestType=CUSTOM&contentType=")) {
            String path = "/ws/rest/v1/session/s/wado-uri";
            var result = mvc.perform(get(URI.create(path+"?"+query))).andReturn().getResponse();
            output.add(record("uri-"+output.size(),path,query,result,captured.get()));
        }
        for (int code : new int[]{204, 206, 401, 403, 404, 429, 500, 503}) {
            status.set(code);
            var result = mvc.perform(get(root)).andReturn().getResponse();
            int expected = code;
            if (result.getStatus() != expected) throw new AssertionError("upstream "+code);
            output.add(record("upstream-"+code,root,"",result,captured.get()));
        }
        status.set(200);
        for (String path : List.of(root.replace("1.2.3","outside"), root.replace("/s/","/missing/"))) {
            captured.set(null);
            var result = mvc.perform(get(path)).andReturn().getResponse();
            if(result.getStatus()!=500 || captured.get()!=null) throw new AssertionError("scoped session null");
            output.add(record("scope-"+output.size(),path,"",result,null));
        }
        // Missing required WADO-URI params are resolved by the real Spring advice.
        String uriPath = "/ws/rest/v1/session/s/wado-uri";
        var missing = mvc.perform(get(uriPath+"?studyUID=1.2.3")).andReturn().getResponse();
        output.add(record("uri-required",uriPath,"studyUID=1.2.3",missing,null));
        includeMedia.set(false);
        for(String accept : List.of("*/*","image/jpeg","multipart/related; transfer-syntax=*")) {
            var result = mvc.perform(get(root+"/series/4.5/instances/6.7").header("Accept",accept)).andReturn().getResponse();
            var row = record("no-media-"+output.size(),root+"/series/4.5/instances/6.7","",result,captured.get());
            row.put("accept",accept);row.put("upstreamMedia",false);output.add(row);
        }
        var mapper = new ObjectMapper();
        Files.writeString(Path.of(args[0]),mapper.writerWithDefaultPrettyPrinter().writeValueAsString(output)+"\n");
        System.out.println("Java Step11 baseline: "+output.size()+" cases");
    }
    static Map<String,Object> record(String name, String path, String query,
        org.springframework.mock.web.MockHttpServletResponse result, Request upstream) {
        var row = new LinkedHashMap<String,Object>();
        row.put("name",name); row.put("path",path); row.put("query",query); row.put("status",result.getStatus());
        row.put("bodyBase64",Base64.getEncoder().encodeToString(result.getContentAsByteArray()));
        var headers = new TreeMap<String,Object>();
        result.getHeaderNames().forEach(key -> headers.put(key.toLowerCase(Locale.ROOT),result.getHeaders(key)));
        row.put("headers",headers);
        if(upstream!=null) {
            row.put("upstreamUrl",upstream.url());
            var requestHeaders = new TreeMap<String,Object>();
            upstream.headers().forEach((key,value)->requestHeaders.put(key.toLowerCase(Locale.ROOT),value));
            row.put("upstreamHeaders",requestHeaders);
        }
        return row;
    }
}
