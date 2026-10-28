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

    public static String downloadSeries(String json,int max) { return "[]"; }
    public static String downloadMetadata(String json,String pattern,String index,int max) { return "[]"; }
    public static void encodeVideo(String source,String destination,int fps,long input,long output,long pixels,int frames) throws Exception {
        String text=java.nio.file.Files.readString(java.nio.file.Path.of(source));
        if(!text.equals("slow") && !text.equals("throw") && !text.isEmpty()) {
            String first=text.lines().findFirst().orElse("");
            text=new String(java.nio.file.Files.readAllBytes(java.nio.file.Path.of(first)),java.nio.charset.StandardCharsets.UTF_8);
        }
        if(text.equals("throw")) throw new IllegalArgumentException("PRIVATE_PATIENT_TOKEN");
        if(text.equals("slow")) {
            java.nio.file.Files.writeString(java.nio.file.Path.of(source+".started"),"started");
            Thread.sleep(700);
        }
        java.nio.file.Files.writeString(java.nio.file.Path.of(destination),text.isEmpty()?"":"fixture-mp4");
    }

    static final class BrokenMessage extends RuntimeException {
        @Override public String getMessage() {
            throw new IllegalStateException("PRIVATE_PATIENT_TOKEN inspection failure");
        }
    }
}
