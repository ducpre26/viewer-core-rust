import java.awt.image.BufferedImage;
import java.nio.file.Path;
import org.apache.commons.imaging.Imaging;
import org.dcm4che3.data.Attributes;
import org.dcm4che3.data.Tag;

/** Probe the original Java raster conversion without opening a printer connection. */
public final class PrintBaseline {
    public static void main(String[] args) throws Exception {
        Class<?> source = Class.forName("vn.saolasoft.viewer.module.printer.service.PrintDicomServiceImpl");
        Object service = source.getConstructors()[0].newInstance(new Object[] { null });
        var convert = source.getDeclaredMethod("storeRasterInDicom", BufferedImage.class, Attributes.class, Boolean.class);
        convert.setAccessible(true);
        BufferedImage image = Imaging.getBufferedImage(Path.of(args[0]).toFile());
        for (boolean color : new boolean[] { false, true }) {
            Attributes dataset = new Attributes();
            convert.invoke(service, image, dataset, color);
            byte[] pixels = dataset.getBytes(Tag.PixelData);
            System.out.println((color ? "color" : "grayscale") + "=" + (pixels == null ? 0 : pixels.length));
        }
    }
}
