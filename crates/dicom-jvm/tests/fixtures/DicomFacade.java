package vn.saolasoft.pacs.dicom;

/** Test-only artifact. Never included in the production facade distribution. */
public final class DicomFacade {
    private static final String MODE = System.getProperty("fixture.mode",
            System.getenv().getOrDefault("JNI_TEST_CASE", "normal"));
    private static int calls;

    static {
        if (MODE.equals("java-thread")) {
            Thread thread = new Thread(() -> { for (;;) java.util.concurrent.locks.LockSupport.park(); });
            thread.setDaemon(false);
            thread.start();
        }
    }

    public static int apiVersion() {
        calls++;
        if (MODE.equals("hang-startup") || (MODE.equals("hang-probe") && calls > 1)) {
            for (;;) java.util.concurrent.locks.LockSupport.park();
        }
        if (MODE.equals("slow") && calls > 1) {
            try { Thread.sleep(500); } catch (InterruptedException error) { throw new RuntimeException(error); }
        }
        if (MODE.equals("throw-probe") && calls > 1) {
            throw new IllegalStateException("PRIVATE_PATIENT_TOKEN /sensitive/path password=secret");
        }
        return MODE.equals("version") ? 2 : 1;
    }

    public static String capabilitiesJson() {
        return switch (MODE) {
            case "json" -> "{invalid";
            case "capabilities" -> "{\"apiVersion\":1,\"operations\":[\"print\"],\"codecs\":[]}";
            case "null" -> null;
            case "large" -> "x".repeat(4097);
            default -> "{\"apiVersion\":1,\"operations\":[],\"codecs\":[]}";
        };
    }

    public static void syntheticError(boolean brokenMessage) {
        if (brokenMessage) { throw new BrokenMessage(); }
        throw new IllegalArgumentException("PRIVATE_PATIENT_TOKEN /sensitive/path password=secret");
    }

    static final class BrokenMessage extends RuntimeException {
        @Override public String getMessage() {
            throw new IllegalStateException("PRIVATE_PATIENT_TOKEN inspection failure");
        }
    }
}
