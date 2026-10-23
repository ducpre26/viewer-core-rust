package vn.saolasoft.pacs.dicom;

/** Stable entry point for the Rust JNI adapter. No DICOM operations yet. */
public final class DicomFacade {
    private DicomFacade() {
    }

    /** Version of the facade contract, independent of its artifact version. */
    public static int apiVersion() {
        return 1;
    }

    /** Only implemented and verified operations/codecs may be advertised. */
    public static String capabilitiesJson() {
        return "{\"apiVersion\":1,\"operations\":[],\"codecs\":[]}";
    }
}
