// Runs against the existing Java target/classes and test dependency classpath.
// Synthetic fixtures only; never changes Java sources or contacts a PACS.
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.annotation.JsonInclude;
import java.nio.file.*;
import java.io.*;
import java.net.URI;
import java.util.*;
import feign.Response;
import jakarta.validation.Validation;
import vn.saolasoft.viewer.module.study.rest.StudyAPI;
import vn.saolasoft.viewer.module.study.service.*;
import vn.saolasoft.viewer.module.study.service.dto.*;
import vn.saolasoft.viewer.module.session.service.SessionService;
import vn.saolasoft.viewer.module.session.service.dto.SessionGetDto;
import vn.saolasoft.viewer.module.qido.service.*;
import vn.saolasoft.viewer.module.qido.feign.QidoProxy;
import vn.saolasoft.viewer.util.DicomWebUtil;
import static org.mockito.Mockito.*;
import static org.mockito.ArgumentMatchers.*;

public class Step10Baseline {
    public static void main(String[] args) throws Exception {
        ObjectMapper mapper = new ObjectMapper().setSerializationInclusion(JsonInclude.Include.NON_NULL);
        SessionService sessions = mock(SessionService.class);
        SessionGetDto session = new SessionGetDto();
        session.setStoreUrl("http://fixture.invalid/dicom");
        session.setStoreAuth("Basic fixture-secret");
        when(sessions.getSession("s")).thenReturn(session);
        QidoProxy proxy = mock(QidoProxy.class);
        QidoService qido = new QidoServiceImpl(proxy);
        StudyService service = new StudyServiceImpl(sessions,qido);
        StudyAPI api = new StudyAPI(service);
        List<Object> output = new ArrayList<>();
        String[] documents = {"[]", "", "broken", "[{\"00100020\":{\"vr\":\"LO\",\"Value\":[\"P-FIXTURE\"]},\"00100010\":{\"vr\":\"PN\",\"Value\":[{\"Alphabetic\":\"Fixture^A\",\"Ideographic\":\"Fixture^B\",\"Phonetic\":\"Fixture^C\"}]},\"00080061\":{\"vr\":\"CS\",\"Value\":[\"CT\",\"MR\"]}}]", "{}", "[null]", "[{\"00100020\":{\"vr\":\"LO\",\"Value\":[\"\"]}}]", "[{\"00100020\":{\"Value\":[\"bad\"]}}]"};
        var request = feign.Request.create(feign.Request.HttpMethod.GET,"http://fixture.invalid/dicom/studies",Map.of(),null,java.nio.charset.StandardCharsets.UTF_8,null);
        for (int i=0;i<documents.length;i++) {
            final Map<String,String> captured = new TreeMap<>();
            final String document = documents[i];
            when(proxy.searchStudy(any(URI.class),any(),eq("studies"),anyMap())).thenAnswer(call -> {
                captured.putAll(call.getArgument(3));
                return Response.builder().request(request).status(200).reason("Fixture").headers(Map.of()).body(document,java.nio.charset.StandardCharsets.UTF_8).build();
            });
            StudyFilter f = new StudyFilter(); f.setSessionID("s"); f.setPatientID("");
            f.setStartDate("20260101");
            var response = api.searchStudy(f,"ignored",-7,0);
            var body = mapper.valueToTree(response.getBody());
            ((com.fasterxml.jackson.databind.node.ObjectNode)body.get("header")).remove("datetime");
            output.add(Map.of("name","document-"+i,"input",document,"status",response.getStatusCode().value(),"body",body,"query",Map.copyOf(captured)));
        }
        StudyFilter missing = new StudyFilter(); missing.setSessionID("missing");missing.setPatientID("p");
        if (api.searchStudy(missing,null,0,100).getStatusCode().value()!=204) throw new AssertionError("missing session");
        try(var factory = Validation.buildDefaultValidatorFactory()) {
            var validator = factory.getValidator();
            StudyFilter f = new StudyFilter();f.setSessionID("s");f.setPatientID("");
            if (!validator.validate(f).isEmpty()) throw new AssertionError("empty patientID allowed");
            f.setPatientID(null);if(validator.validate(f).isEmpty())throw new AssertionError("null patientID rejected");
            f.setPatientID("p");f.setSessionID("");if(validator.validate(f).isEmpty())throw new AssertionError("empty session rejected");
        }
        var mvc = org.springframework.test.web.servlet.setup.MockMvcBuilders.standaloneSetup(api)
            .setControllerAdvice(new vn.saolasoft.viewer.exception.GlobalExceptionHandler()).build();
        for (String query : List.of("", "?offset=&limit=", "?offset=x", "?offset=2147483648", "?limit=-1")) {
            var response = mvc.perform(org.springframework.test.web.servlet.request.MockMvcRequestBuilders.post("/ws/rest/v1/study/search"+query)
                .contentType("application/json").content("{\"sessionID\":\"s\",\"patientID\":\"p\"}")).andReturn().getResponse();
            System.out.println("query-case "+query+" status="+response.getStatus());
            int expected = query.contains("=x") || query.contains("2147483648") ? 500 : 200;
            if(response.getStatus()!=expected)throw new AssertionError("query "+query);
        }
        var options = new feign.Request.Options();
        for (String doc : List.of("{\"00100020\":{\"vr\":\"LO\",\"Value\":[\"p\"]}}", "[{}]", "[{\"00100020\":{\"vr\":\"LO\",\"Value\":[\" p  \"]}}]", "[{\"00100010\":{\"vr\":\"PN\",\"Value\":[{}]}}]")) {
            var attrs = DicomWebUtil.parseJSON(new ByteArrayInputStream(doc.getBytes(java.nio.charset.StandardCharsets.UTF_8)));
            System.out.println("edge="+doc+" count="+attrs.size()+" pid="+(attrs.isEmpty()?"none":attrs.getFirst().getString(0x00100020))+" pn="+(attrs.isEmpty()?"none":attrs.getFirst().getString(0x00100010)));
        }
        System.out.println("Feign connect/read ms: "+options.connectTimeoutMillis()+"/"+options.readTimeoutMillis()+" redirects="+options.isFollowRedirects());
        Files.writeString(Path.of(args[0]),mapper.writerWithDefaultPrettyPrinter().writeValueAsString(output)+"\n");
        System.out.println("Java Step10 baseline: "+output.size()+" datasets; missing-session and Bean Validation checked");
    }
}
