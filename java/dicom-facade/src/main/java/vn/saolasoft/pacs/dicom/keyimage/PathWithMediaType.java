package vn.saolasoft.pacs.dicom.keyimage;
import java.nio.file.*; import java.io.*; import jakarta.ws.rs.core.MediaType;
final class PathWithMediaType {
 private final Path path; private final long length; private final MediaType mediaType;
 PathWithMediaType(Path p,MediaType m)throws IOException{path=p;length=Files.size(p);mediaType=m;}
 Path getPath(){return path;} long getLength(){return length;} MediaType getMediaType(){return mediaType;}
}
