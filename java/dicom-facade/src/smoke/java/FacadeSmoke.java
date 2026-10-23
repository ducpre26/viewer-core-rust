import java.nio.file.Path;
import vn.saolasoft.pacs.dicom.DicomFacade;

/** Compiled and run against the packaged JAR by tools/facade.py. */
public final class FacadeSmoke {
    public static void main(String[] args) throws Exception {
        if (Runtime.version().feature() != 25) {
            throw new IllegalStateException("Java 25 required");
        }
        Path loadedFrom = Path.of(DicomFacade.class.getProtectionDomain()
                .getCodeSource().getLocation().toURI()).toRealPath();
        if (!loadedFrom.equals(Path.of(args[0]).toRealPath())) {
            throw new IllegalStateException("Facade was not loaded from the packaged JAR");
        }
        if (DicomFacade.apiVersion() != 1) {
            throw new IllegalStateException("Unexpected facade API version");
        }
        System.out.println(DicomFacade.capabilitiesJson());
        System.out.println(System.getProperty("java.vendor"));
        System.out.println(System.getProperty("java.runtime.version"));
        System.out.println(System.getProperty("os.name"));
        System.out.println(System.getProperty("os.arch"));
    }
}
