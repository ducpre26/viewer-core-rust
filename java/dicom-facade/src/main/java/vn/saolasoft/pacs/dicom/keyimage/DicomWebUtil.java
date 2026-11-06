// Ported from Java revision 2373896f; DICOM behavior retained, no Spring/business persistence.
package vn.saolasoft.pacs.dicom.keyimage;

import jakarta.json.Json;
import jakarta.json.stream.JsonGenerator;
import org.dcm4che3.data.Attributes;
import org.dcm4che3.json.JSONReader;
import org.dcm4che3.json.JSONWriter;

import java.io.*;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

public class DicomWebUtil {

    static public List<Attributes> parseJSON(InputStream in) throws IOException {
        try {
            JSONReader jsonReader = new JSONReader(Json.createParser(new InputStreamReader(in, "UTF-8")));
            List<Attributes> result = new ArrayList<>();
            jsonReader.readDatasets((fmi, attrs) -> {
                result.add(attrs);
            });
            return result;
        } catch (Exception ignored) {
            return Collections.emptyList();
        }

    }

    static public InputStream writeAttributes(List<Attributes> attributes) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        JsonGenerator gen = Json.createGenerator(out);
        JSONWriter writer = new JSONWriter(gen);
        gen.writeStartArray();

        for (Attributes attrs : attributes) {
            writer.write(attrs);
        }
        gen.writeEnd();
        gen.flush();
        return new ByteArrayInputStream(out.toByteArray());
    }

}
