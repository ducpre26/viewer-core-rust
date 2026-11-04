package vn.saolasoft.pacs.dicom;

import java.io.*;
import java.nio.file.*;
import java.awt.image.BufferedImage;
import java.util.*;
import javax.imageio.*;
import jakarta.json.Json;
import org.dcm4che3.data.*;
import org.dcm4che3.io.DicomInputStream;
import org.dcm4che3.json.JSONReader;
import org.dcm4che3.util.AttributesFormat;
import org.bytedeco.javacv.*;
import static org.bytedeco.ffmpeg.global.avcodec.AV_CODEC_ID_H264;
import static org.bytedeco.ffmpeg.global.avutil.AV_PIX_FMT_YUV420P;

/** Typed file operations. Rust owns transport, jobs, file leases and ZIP packaging. */
public final class DicomFacade {
    private DicomFacade() {}
    public static int apiVersion() { return 2; }
    public static String capabilitiesJson() {
        // Native codecs are not advertised until tested on the deployed platform.
        return "{\"apiVersion\":2,\"operations\":[\"downloadMetadata\",\"downloadSeries\",\"encodeVideo\"],\"codecs\":[]}";
    }
    private static final String DEFAULT_PATH = "{00100020}-{00100010}/{00080050}/{00200011}-{0008103E}/{00080018}";

    private static List<Attributes> datasets(String json) throws IOException {
        var values = new ArrayList<Attributes>();
        try (var parser = Json.createParser(new StringReader(json))) {
            new JSONReader(parser).readDatasets((meta, attrs) -> {
                values.add(attrs);
            });
        } catch (Exception e) { values.clear(); } // DicomWebUtil malformed JSON behavior
        return values;
    }
    public static String downloadSeries(String json) throws IOException {
        var values = datasets(json);
        var map = new HashMap<String,String>();
        for (int i=0; i<values.size(); i++) {
            var attrs = values.get(i);
            String key = Integer.toString(attrs.getInt(Tag.SeriesNumber,i));
            if (map.get(key)!=null) key += "."+i;
            map.put(key,attrs.getString(Tag.SeriesInstanceUID));
        }
        var result = Json.createArrayBuilder();
        for (var e: map.entrySet()) result.add(Json.createObjectBuilder()
            .add("sop",Objects.toString(e.getValue(), "")).add("index",e.getKey()));
        return result.build().toString();
    }
    public static String downloadMetadata(String json, String pattern, String seriesIndex) throws IOException {
        AttributesFormat format;
        try { format = new AttributesFormat(pattern.isEmpty()?DEFAULT_PATH:pattern); }
        catch (Exception e) { format = new AttributesFormat(DEFAULT_PATH); }
        var values = datasets(json);
        var map = new LinkedHashMap<String,Attributes>();
        for(int i=0;i<values.size();i++) {
            var a = values.get(i);
            String key = Integer.toString(a.getInt(Tag.InstanceNumber,i));
            if(map.containsKey(key)) key += "."+i;
            map.put(key,a);
        }
        var result = Json.createArrayBuilder();
        for(var a: map.values()) {
            a.setString(Tag.PatientID,VR.SH,a.getString(Tag.PatientID,"undefined"));
            a.setString(Tag.PatientName,VR.PN,a.getString(Tag.PatientName,"undefined"));
            a.setString(Tag.AccessionNumber,VR.SH,a.getString(Tag.AccessionNumber,"undefined"));
            a.setString(Tag.SeriesNumber,VR.IS,a.getString(Tag.SeriesNumber,seriesIndex));
            a.setString(Tag.SeriesDescription,VR.LO,a.getString(Tag.SeriesDescription,"undefined"));
            result.add(Json.createObjectBuilder().add("sop",Objects.toString(a.getString(Tag.SOPInstanceUID),""))
                .add("frames",a.getInt(Tag.NumberOfFrames,1)).add("path",format.format(a)));
        }
        return result.build().toString();
    }
    private static Path input(String name) throws IOException {
        Path path = Path.of(name).toRealPath();
        if(!Files.isRegularFile(path)) throw new IOException("Invalid input file");
        return path;
    }
    private static OutputStream output(String name, Path parent) throws IOException {
        Path path = Path.of(name);
        if(!path.getParent().toRealPath().equals(parent.toRealPath())) throw new IOException("Output sandbox");
        return Files.newOutputStream(path,StandardOpenOption.CREATE_NEW);
    }
    private static Attributes inspect(Path path) throws IOException {
        try(var stream = new DicomInputStream(path.toFile())) {
            var a = stream.readDatasetUntilPixelData();
            long width=a.getInt(Tag.Columns,0), height=a.getInt(Tag.Rows,0);
            int count=a.getInt(Tag.NumberOfFrames,1);
            if(count<=0 || width<=0 || height<=0) throw new IOException("Invalid image dimensions or frames");
            return a;
        }
    }
    public static void encodeVideo(String manifest,String destination,int fps) throws Exception {
        Path list=input(manifest);
        if(fps<=0) throw new IOException("Invalid video FPS");
        FFmpegFrameRecorder recorder=null;int width=0,height=0;
        try(var out=output(destination,list.getParent());
            var lines=Files.newBufferedReader(list);
            var converter=new Java2DFrameConverter()) {
            for(String line;(line=lines.readLine())!=null;) {
                Path source=input(line);
                if(!source.getParent().equals(list.getParent())) throw new IOException("Video sandbox");
                var attrs=inspect(source);int count=attrs.getInt(Tag.NumberOfFrames,1);
                var readers=ImageIO.getImageReadersByFormatName("DICOM");
                if(!readers.hasNext()) throw new IOException("DICOM ImageIO unavailable");
                var reader=readers.next();
                try(var imageInput=ImageIO.createImageInputStream(source.toFile())) {
                    reader.setInput(imageInput);
                    for(int frame=0;frame<count;frame++) {
                        BufferedImage image=reader.read(frame,reader.getDefaultReadParam());
                        if(recorder==null) {
                            width=image.getWidth();height=image.getHeight();
                            recorder=new FFmpegFrameRecorder(out,width,height);
                            recorder.setVideoCodec(AV_CODEC_ID_H264);recorder.setFormat("mp4");
                            recorder.setPixelFormat(AV_PIX_FMT_YUV420P);recorder.setFrameRate(fps);
                            recorder.setVideoBitrate(400000);
                            recorder.setOption("movflags","frag_keyframe+empty_moov+faststart");
                            recorder.setOption("preset","ultrafast");recorder.setOption("tune","zerolatency");
                            recorder.start();
                        }
                        if(image.getWidth()!=width || image.getHeight()!=height) throw new IOException("Video dimensions differ");
                        recorder.record(converter.convert(image));image.flush();
                    }
                } finally { reader.dispose(); }
            }
            if(recorder!=null) recorder.stop();
        } finally { if(recorder!=null) recorder.release(); }
    }
}
