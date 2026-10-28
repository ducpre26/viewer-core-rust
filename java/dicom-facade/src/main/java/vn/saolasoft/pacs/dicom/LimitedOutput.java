package vn.saolasoft.pacs.dicom;
import java.io.*;

/** Bounds bytes written to disk, including native recorder callbacks. */
final class LimitedOutput extends FilterOutputStream {
    private long remaining;
    private boolean exceeded;
    LimitedOutput(OutputStream output,long limit) { super(output);remaining=limit; }
    @Override public void write(int b) throws IOException {
        if(remaining<1) { exceeded=true;throw new IOException("Output limit"); }
        out.write(b);remaining--;
    }
    @Override public void write(byte[] bytes,int offset,int length) throws IOException {
        if(length>remaining) { exceeded=true;throw new IOException("Output limit"); }
        out.write(bytes,offset,length);remaining-=length;
    }
    @Override public void close() throws IOException {
        try { out.close(); } finally { if(exceeded) throw new IOException("Output limit"); }
    }
}
