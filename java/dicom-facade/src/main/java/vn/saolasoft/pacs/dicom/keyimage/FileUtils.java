// Ported from Java revision 2373896f; DICOM behavior retained, no Spring/business persistence.
package vn.saolasoft.pacs.dicom.keyimage;

import java.io.File;
import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.Date;

public class FileUtils {

    private FileUtils() {
    }

    public static File createDirIfNotExist(String url) throws IOException {
        File folder = new File(url);
        if (!folder.exists()) {
            if (folder.mkdirs()) {
                if (!folder.setWritable(true))
                    throw new IOException("Could not set Writable permission to folder '" + url + "'");
                if (!folder.setReadable(true))
                    throw new IOException("Could not set Readable permission to folder '{" + url + "}'");
                return folder;
            } else return null;
        }
        return folder;
    }
    public static String createStoredDir(String root, String folderName) throws IOException {
        if (FileUtils.createDirIfNotExist(root) == null) throw new IOException("Error creating pdfPath folder");

        if (FileUtils.createDirIfNotExist(root + "/" + folderName) == null) throw new IOException("Error creating folder");

        return root + "/" + folderName;
    }

    public static void purgeDirectory(String path) {
        try {
            Path spoolDirectory = Paths.get(path);
            Files.walkFileTree(spoolDirectory, new SimpleFileVisitor<>() {
                public FileVisitResult visitFile(Path file, BasicFileAttributes attrs) throws IOException {
                    Files.delete(file);
                    return FileVisitResult.CONTINUE;
                }

                public FileVisitResult postVisitDirectory(Path dir, IOException exc) throws IOException {
                    Files.delete(dir);
                    return FileVisitResult.CONTINUE;
                }
            });
        } catch (IOException ignored) { }
    }
}
