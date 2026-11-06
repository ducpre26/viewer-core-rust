// Ported DICOM branches from StowRSServiceImpl at 2373896f. Rust owns DB/session/STOW.
package vn.saolasoft.pacs.dicom.keyimage;

import jakarta.ws.rs.core.MediaType;
import org.dcm4che3.data.*;
import org.dcm4che3.image.BufferedImageUtils;
import org.dcm4che3.io.DicomInputStream;
import org.dcm4che3.mime.MultipartInputStream;
import org.dcm4che3.mime.MultipartParser;
import org.dcm4che3.util.ByteUtils;
import org.dcm4che3.util.StreamUtils;
import org.dcm4che3.util.TagUtils;
import org.dcm4che3.util.UIDUtils;
import org.dcm4che3.ws.rs.MediaTypes;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import javax.imageio.ImageIO;
import javax.imageio.ImageReader;
import javax.imageio.stream.ImageInputStream;
import java.io.*;
import java.nio.file.StandardCopyOption;
import java.io.File;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.channels.SeekableByteChannel;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

public final class KeyImageSupport {

    private final Logger logger = LoggerFactory.getLogger(this.getClass());

    private final int[] IMAGE_PIXEL_TAGS = {
            Tag.SamplesPerPixel,
            Tag.PhotometricInterpretation,
            Tag.Rows,
            Tag.Columns,
            Tag.BitsAllocated,
            Tag.BitsStored,
            Tag.HighBit,
            Tag.PixelRepresentation
    };

    private final List<String> encapsulatedDocument = Arrays.asList(
            UID.EncapsulatedPDFStorage, UID.EncapsulatedCDAStorage, UID.EncapsulatedSTLStorage,
            UID.EncapsulatedOBJStorage, UID.EncapsulatedMTLStorage, UID.PrivateDcm4cheEncapsulatedGenozipStorage,
            UID.PrivateDcm4cheEncapsulatedBzip2VCFStorage, UID.PrivateDcm4cheEncapsulatedBzip2DocumentStorage
    );

    private final ElementDictionary DICT = ElementDictionary.getStandardElementDictionary();

    private final FileStorageServiceImpl fileStorageService;
    private KeyImageSupport(String storage){fileStorageService=new FileStorageServiceImpl(storage);}
    private String getBoundary(String contentType) {
        if (contentType == null) {
            logger.error("Key image DICOM operation");
            return null;
        }

        MediaType mediaType = MediaType.valueOf(contentType);
        if (!mediaType.isCompatible(MediaTypes.MULTIPART_RELATED_TYPE)) {
            logger.error("Key image DICOM operation");
            return null;
        }

        Map<String, String> parameters = mediaType.getParameters();
        if (parameters == null || parameters.isEmpty() || !parameters.containsKey("boundary")) {
            logger.error("Key image DICOM operation");
            return null;
        }
        return parameters.get("boundary");
    }

    private void supplementSeriesMetadata(List<Attributes> instances, String retrieveURL) {
        instances.forEach(attrs -> {
            StringBuffer sb = new StringBuffer(retrieveURL);
            mkInstanceURL(sb, attrs);
            attrs.setValue(Tag.PixelData, VR.OB, new BulkData(null, "", attrs.bigEndian()));
            setBulkdataURI(attrs, sb.toString());
        });
    }

    private void mkInstanceURL(StringBuffer sb, Attributes attributes) {
        sb.append('/').append(attributes.getString(Tag.StudyInstanceUID));
        if (sb.lastIndexOf("/instances/") < 0) {
            if (sb.lastIndexOf("/series/") < 0) {
                sb.append("/series/").append(attributes.getString(Tag.SeriesInstanceUID));
            }
            sb.append("/instances/").append(attributes.getString(Tag.SOPInstanceUID));
        }
    }

    private void setBulkdataURI(Attributes attrs, String retrieveURL) {
        try {
            attrs.accept(new Attributes.ItemPointerVisitor() {
                @Override
                public boolean visit(Attributes attrs, int tag, VR vr, Object value) {
                    if (value instanceof BulkData bulkData) {
                        if (tag == Tag.PixelData && itemPointers.isEmpty()) {
                            bulkData.setURI(retrieveURL);
                        } else {
                            bulkData.setURI(retrieveURL + "/bulkdata"
                                    + DicomInputStream.toAttributePath(itemPointers, tag));
                        }
                    }
                    return true;
                }
            }, true);
        } catch (Exception e) {
            throw new KeyImageFailure(KeyImageError.GLOBAL_ERROR, e.getMessage());
        }
    }

    private void storeDicomObject(Attributes attrs, int instanceNumber, Map<String, PathWithMediaType> bulkdataMap) {
        TransferSyntaxDTO transferSyntax = new TransferSyntaxDTO(UID.ExplicitVRLittleEndian);
        PathWithMediaType pathWithMediaType = resolveBulkdataRefs(attrs, bulkdataMap);
        if (pathWithMediaType != null) {
            transferSyntax.setUid(MediaTypes.transferSyntaxOf(pathWithMediaType.getMediaType()));
            supplementAttrs(transferSyntax, attrs, instanceNumber, pathWithMediaType);
        }
        supplementKeyImageData(attrs);
        try {
            String sopIUID = attrs.getString(Tag.SOPInstanceUID);
            int frameLength = attrs.getInt(Tag.Rows, 0) * attrs.getInt(Tag.Columns, 0) * attrs.getInt(Tag.SamplesPerPixel, 0) * attrs.getInt(Tag.BitsAllocated, 8) / 8;
            logger.debug("Key image DICOM operation");
            String fileName = fileStorageService.saveKeyImage(transferSyntax.getUid(), attrs);
            logger.info("Key image DICOM operation");

            try (DicomInputStream dis = new DicomInputStream(new File(fileName))) {
                Attributes attributes = dis.readDataset();
                int frameLengthAfter = attributes.getInt(Tag.Rows, 0) * attributes.getInt(Tag.Columns, 0) * attributes.getInt(Tag.SamplesPerPixel, 0) * attributes.getInt(Tag.BitsAllocated, 8) / 8;
                logger.debug("Key image DICOM operation");
            }
        } catch (IOException e) {
            logger.error("Key image DICOM operation");
            throw new KeyImageFailure(KeyImageError.SAVE_FILE_ERROR);
        }
    }

    private void supplementAttrs(TransferSyntaxDTO transferSyntax, Attributes attrs, int instanceNumber, PathWithMediaType bulkdata) {
        if (!attrs.containsValue(Tag.StudyInstanceUID)) throw missingAttribute(Tag.StudyInstanceUID);
        if (!attrs.containsValue(Tag.SOPInstanceUID)) attrs.setString(Tag.SOPInstanceUID, VR.UI, UIDUtils.createUID());

        String cuid = attrs.getString(Tag.SOPClassUID);
        if (cuid == null) {
            cuid = MediaTypes.sopClassOf(bulkdata.getMediaType());
            if (cuid == null) throw missingAttribute(Tag.SOPClassUID);
            attrs.setString(Tag.SOPClassUID, VR.UI, cuid);
        }

        if (!attrs.containsValue(Tag.InstanceCreationDate)) attrs.setDate(Tag.InstanceCreationDateAndTime, new Date());
        if (!attrs.containsValue(Tag.SeriesNumber)) attrs.setInt(Tag.SeriesNumber, VR.IS, 999);
        if (!attrs.containsValue(Tag.InstanceNumber)) attrs.setInt(Tag.InstanceNumber, VR.IS, instanceNumber);

        if (attrs.containsValue(Tag.PixelData)) {
            supplementImagePixelModule(transferSyntax, attrs, bulkdata);
            verifyImagePixelModule(attrs);
        }

        if (attrs.containsValue(Tag.EncapsulatedDocument)) verifyEncapsulatedDocumentModule(attrs, bulkdata);
        else if (encapsulatedDocument.contains(cuid)) throw missingAttribute(Tag.EncapsulatedDocument);
    }

    private void supplementKeyImageData(Attributes attrs) {
        if (!attrs.containsValue(Tag.ImageType)) attrs.setString(Tag.ImageType, VR.CS, "SECONDARY");
        if (!attrs.containsValue(Tag.SeriesDescription)) attrs.setString(Tag.SeriesDescription, VR.LO, "KEY IMAGE");
    }

    private void verifyEncapsulatedDocumentModule(Attributes attrs, PathWithMediaType bulkdata) {
        String cuid = attrs.getString(Tag.SOPClassUID);
        if (!attrs.containsValue(Tag.MIMETypeOfEncapsulatedDocument)) {
            attrs.setString(Tag.MIMETypeOfEncapsulatedDocument, VR.LO, bulkdata.getMediaType().toString());
        }
        if (!attrs.containsValue(Tag.BurnedInAnnotation)) attrs.setString(Tag.BurnedInAnnotation, VR.CS, "YES");
        if (!attrs.containsValue(Tag.EncapsulatedDocumentLength))
            attrs.setInt(Tag.EncapsulatedDocumentLength, VR.UL, (int) bulkdata.getLength());

        switch (cuid) {
            case UID.EncapsulatedSTLStorage, UID.EncapsulatedMTLStorage, UID.EncapsulatedOBJStorage -> {
                if (!attrs.contains(Tag.MeasurementUnitsCodeSequence)) {
                    Attributes item = new Attributes(3);
                    item.setString(Tag.CodeValue, VR.SH, "mm");
                    item.setString(Tag.CodingSchemeDesignator, VR.SH, "UCUM");
                    item.setString(Tag.CodeMeaning, VR.LO, "mm");
                    attrs.newSequence(Tag.MeasurementUnitsCodeSequence, 1).add(item);
                }
                if (!attrs.containsValue(Tag.Modality)) attrs.setString(Tag.Modality, VR.CS, "M3D");
                if (!attrs.containsValue(Tag.Manufacturer)) attrs.setString(Tag.Manufacturer, VR.LO, "UNKNOWN");
                if (!attrs.containsValue(Tag.ManufacturerModelName))
                    attrs.setString(Tag.ManufacturerModelName, VR.LO, "UNKNOWN");
                if (!attrs.containsValue(Tag.DeviceSerialNumber))
                    attrs.setString(Tag.DeviceSerialNumber, VR.LO, "UNKNOWN");
                if (!attrs.containsValue(Tag.SoftwareVersions)) attrs.setString(Tag.SoftwareVersions, VR.LO, "UNKNOWN");
                if (!attrs.containsValue(Tag.FrameOfReferenceUID) && !cuid.equals(UID.EncapsulatedMTLStorage))
                    attrs.setString(Tag.FrameOfReferenceUID, VR.UI, UIDUtils.createUID());
            }
            case UID.EncapsulatedPDFStorage -> {
                if (!attrs.containsValue(Tag.Modality)) attrs.setString(Tag.Modality, VR.CS, "DOC");
                if (!attrs.containsValue(Tag.ConversionType)) attrs.setString(Tag.ConversionType, VR.CS, "SD");
            }
            case UID.EncapsulatedCDAStorage -> {
                if (!attrs.containsValue(Tag.Modality)) attrs.setString(Tag.Modality, VR.CS, "SR");
                if (!attrs.containsValue(Tag.ConversionType)) attrs.setString(Tag.ConversionType, VR.CS, "WSD");
            }
        }
    }

    private void supplementImagePixelModule(TransferSyntaxDTO transferSyntax, Attributes attrs,
                                            PathWithMediaType bulkdata) {
        CompressedPixelData compressedPixelData = CompressedPixelData.valueOf(bulkdata.getMediaType());
        ImageReader imageReader;
        if (compressedPixelData != null) {
            try (SeekableByteChannel channel = Files.newByteChannel(bulkdata.getPath())) {
                transferSyntax.setUid(compressedPixelData.supplementImagePixelModule(channel, attrs, false));
            } catch (IOException e) {
                logger.info("Key image DICOM operation");
                throw new KeyImageFailure(KeyImageError.SUPPLEMENT_ERROR);
            }
        } else if ((imageReader = findImageReader(bulkdata.getMediaType(), "com.sun.imageio")) != null) {
            try (ImageInputStream iio = ImageIO.createImageInputStream(bulkdata.getPath().toFile())) {
                imageReader.setInput(iio);
                BufferedImageUtils.toImagePixelModule(imageReader, attrs);
                transferSyntax.setUid(UID.ExplicitVRLittleEndian);
            } catch (Exception e) {
                logger.info("Key image DICOM operation");
                throw new KeyImageFailure(KeyImageError.SUPPLEMENT_ERROR);
            } finally {
                imageReader.dispose();
            }
        }
    }


    private ImageReader findImageReader(MediaType mediaType, String pkg) {
        Iterator<ImageReader> iter = ImageIO.getImageReadersByMIMEType(mediaType.toString());
        if (iter.hasNext()) {
            ImageReader reader = iter.next();
            while (!reader.getClass().getName().startsWith(pkg) && iter.hasNext()) {
                reader = iter.next();
            }
            return reader;
        } else {
            return null;
        }
    }

    private void verifyImagePixelModule(Attributes attrs) {
        for (int tag : IMAGE_PIXEL_TAGS)
            if (!attrs.containsValue(tag)) throw missingAttribute(tag);
        if (attrs.getInt(Tag.SamplesPerPixel, 1) > 1 && !attrs.containsValue(Tag.PlanarConfiguration))
            throw missingAttribute(Tag.PlanarConfiguration);
    }

    private KeyImageFailure missingAttribute(int tag) {
        return new KeyImageFailure(KeyImageError.MISSING_TAG,
                "Missing " + DICT.keywordOf(tag) + " " + TagUtils.toString(tag));
    }

    private PathWithMediaType resolveBulkdataRefs(Attributes attrs, Map<String, PathWithMediaType> bulkdataMap) {
        final PathWithMediaType[] bulkdataWithMediaType = new PathWithMediaType[1];
        try {
            attrs.accept((attrs1, tag, vr, value) -> {
                if (value instanceof BulkData)
                    bulkdataWithMediaType[0] = resolveBulkdataRef(attrs1, tag, vr, (BulkData) value, bulkdataMap);
                return true;
            }, true);
        } catch (Exception e) {
            throw new KeyImageFailure(KeyImageError.GLOBAL_ERROR, e.getMessage());
        }
        return bulkdataWithMediaType[0];
    }

    private PathWithMediaType resolveBulkdataRef(Attributes attrs, int tag, VR vr, BulkData bulkdata, Map<String, PathWithMediaType> bulkdataMap) {
        PathWithMediaType pathWithMediaType = bulkdataMap.get(bulkdata.getURI());
        if (pathWithMediaType == null) return null;
        if (tag != Tag.PixelData || MediaType.APPLICATION_OCTET_STREAM_TYPE.equals(pathWithMediaType.getMediaType()))
            bulkdata.setURI(pathWithMediaType.getPath().toUri() + "?offset=0&length=" + pathWithMediaType.getLength());
        else addFragments(attrs, pathWithMediaType, tag, vr);
        return pathWithMediaType;
    }

    private void addFragments(Attributes attrs, PathWithMediaType pathWithMediaType,
                              int tag, VR vr) {
        String uri = pathWithMediaType.getPath().toUri().toString();
        long offset = 0;
        long remaining = pathWithMediaType.getLength();
        long maxFragmentLength = Integer.MAX_VALUE - 1;
        Fragments frags = attrs.newFragments(tag, vr, 2 + (int) ((remaining - 1) / maxFragmentLength));
        frags.add(ByteUtils.EMPTY_BYTES);
        while (remaining > maxFragmentLength) {
            frags.add(new BulkData(uri, offset, maxFragmentLength, false));
            offset += maxFragmentLength;
            remaining -= maxFragmentLength;
        }
        frags.add(new BulkData(uri, offset, remaining, false));
    }

    private boolean readBodyPart(Path spoolDirectory, List<Attributes> instances, Map<String, PathWithMediaType> bulkdataMap,
                                 MediaType mediaType, String contentLocation, MultipartInputStream in) throws IOException {
        if (!MediaTypes.equalsIgnoreParameters(mediaType, MediaTypes.APPLICATION_DICOM_JSON_TYPE)) {
            return spoolBulkdata(spoolDirectory, in, bulkdataMap, mediaType, contentLocation);
        }
        instances.addAll(DicomWebUtil.parseJSON(in));
        return true;
    }

    private boolean spoolBulkdata(Path spoolDirectory, MultipartInputStream in, Map<String, PathWithMediaType> bulkdataMap,
                                  MediaType mediaType, String contentLocation) {
        try {
            Path spoolFile = Files.createTempFile(spoolDirectory, null, null);
            try (OutputStream out = Files.newOutputStream(spoolFile)) {
                StreamUtils.copy(in, out);
            }
            bulkdataMap.put(contentLocation, new PathWithMediaType(spoolFile, mediaType));
            return true;
        } catch (IOException e) {
            logger.error("Key image DICOM operation");
            throw new KeyImageFailure(KeyImageError.GLOBAL_ERROR,
                    "IOException caught while spooling bulkdata: " + e.getMessage());
        }
    }

    private MediaType normalize(MediaType mediaType) {
        return MediaTypes.isSTLType(mediaType) ? MediaTypes.MODEL_STL_TYPE : mediaType;
    }

    private String getHeaderParamValue(Map<String, List<String>> headerParams, String key) {
        List<String> list = headerParams.get(key);
        return list != null && !list.isEmpty() ? list.get(0) : null;
    }


    public static String run(String operation, String manifest) throws Exception {
        try { return execute(operation, manifest); }
        catch(KeyImageFailure e) {return jakarta.json.Json.createObjectBuilder().add("status",e.error.status).add("errorCode",e.error.code).add("message",Objects.toString(e.getMessage(),e.error.message)).build().toString();}
        catch(Exception e) {return jakarta.json.Json.createObjectBuilder().add("status",400).add("errorCode","KEY_IMG_GLOBAL_ERROR").add("message",Objects.toString(e.getMessage(),"Key image global error")).build().toString();}
    }
    private static String execute(String operation,String manifest) throws Exception {
        var m=jakarta.json.Json.createReader(new StringReader(manifest)).readObject();
        Path job=Path.of(m.getString("job")).toRealPath();
        var support=new KeyImageSupport(m.getString("storage"));
        if(operation.equals("parse")) {
            var instances=new ArrayList<Attributes>(); var bulk=new HashMap<String,PathWithMediaType>();
            String boundary=support.getBoundary(m.getString("contentType",""));
            if(boundary==null)return "{\"status\":400,\"errorCode\":\"BAD_REQUEST\",\"message\":\"Missing boundary\"}";
            try(var in=Files.newInputStream(job.resolve("request"))) {
                new MultipartParser(boundary).parse(in,(number,part)->{
                    var headers=part.readHeaderParams();
                    String location=support.getHeaderParamValue(headers,"content-location");
                    MediaType type=support.normalize(MediaType.valueOf(support.getHeaderParamValue(headers,"content-type")));
                    try {support.readBodyPart(job,instances,bulk,type,location,part);}
                    catch(Exception e){throw new KeyImageFailure(KeyImageError.GLOBAL_ERROR,e.getMessage());}
                });
            }
            try(var in=DicomWebUtil.writeAttributes(instances)){Files.copy(in,job.resolve("instances.json"),StandardCopyOption.REPLACE_EXISTING);}
            var map=jakarta.json.Json.createArrayBuilder();
            for(var e:bulk.entrySet())map.add(jakarta.json.Json.createObjectBuilder().add("location",Objects.toString(e.getKey(),""))
                .add("path",e.getValue().getPath().getFileName().toString()).add("type",e.getValue().getMediaType().toString()));
            Files.writeString(job.resolve("bulk.json"),map.build().toString());
            var result=jakarta.json.Json.createObjectBuilder().add("count",instances.size());
            if(!instances.isEmpty()) {
                var a=instances.getFirst();
                if(a.getString(Tag.StudyInstanceUID)!=null)result.add("study",a.getString(Tag.StudyInstanceUID));
                if(a.getString(Tag.SeriesInstanceUID)!=null)result.add("series",a.getString(Tag.SeriesInstanceUID));
            }
            return result.build().toString();
        }
        if(operation.equals("write")) {
            List<Attributes> instances;
            try(var in=Files.newInputStream(job.resolve("instances.json"))){instances=DicomWebUtil.parseJSON(in);}
            var bulk=new HashMap<String,PathWithMediaType>();
            try(var reader=jakarta.json.Json.createReader(Files.newBufferedReader(job.resolve("bulk.json")))) {
                for(var v:reader.readArray()) {var e=v.asJsonObject();bulk.put(e.getString("location"),new PathWithMediaType(job.resolve(e.getString("path")),MediaType.valueOf(e.getString("type"))));}
            }
            int index=0;
            for(var a:instances) {a.setString(Tag.SeriesInstanceUID,VR.UI,m.getString("series"));support.storeDicomObject(a,++index,bulk);}
            support.supplementSeriesMetadata(instances,m.getString("retrieve"));support.fileStorageService.saveSeriesMetadata(instances);
            try(var in=DicomWebUtil.writeAttributes(instances)){return new String(in.readAllBytes(),StandardCharsets.UTF_8);}
        }

        if(operation.equals("merge")) {
            List<Attributes> remote;
            try(var in=Files.newInputStream(job.resolve("remote.json"))){remote=DicomWebUtil.parseJSON(in);}
            String series=m.getString("series");
            if(m.getBoolean("listing")) {
                if(remote.stream().noneMatch(a->series.equals(a.getString(Tag.SeriesInstanceUID)))) {
                    var a=new Attributes();a.setInt(Tag.SeriesNumber,VR.IS,999);a.setInt(Tag.NumberOfStudyRelatedSeries,VR.IS,1);
                    a.setString(Tag.StudyInstanceUID,VR.UI,m.getString("study"));a.setString(Tag.SeriesInstanceUID,VR.UI,series);
                    a.setString(Tag.InstanceAvailability,VR.CS,"ONLINE");a.setString(Tag.RetrieveAETitle,VR.AE,"SAOLA");
                    a.setString(Tag.RetrieveURI,VR.UR,m.getString("retrieve")+"/"+series);remote.add(a);
                }
            } else {
                try(var in=support.fileStorageService.getSeriesMetadata(series)) {
                    if(in==null)return "{\"missing\":true}";
                    var map=new HashMap<String,Attributes>();
                    for(var a:remote)map.putIfAbsent(a.getString(Tag.SOPInstanceUID),a);
                    for(var a:DicomWebUtil.parseJSON(in))map.putIfAbsent(a.getString(Tag.SOPInstanceUID),a);
                    remote=new ArrayList<>(map.values());
                }
            }
            try(var in=DicomWebUtil.writeAttributes(remote)){return new String(in.readAllBytes(),StandardCharsets.UTF_8);}
        }
        if(operation.equals("frame")) {
            try(var in=support.fileStorageService.getInstanceFrame(m.getString("series"),m.getString("sop"))) {
                if(in==null)return "{\"missing\":true}";
                Files.copy(in,job.resolve("frame"));
            }
            return "{}";
        }
        throw new IOException("Unknown key image operation");
    }
}
