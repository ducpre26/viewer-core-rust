package vn.saolasoft.pacs.dicom.keyimage;
final class KeyImageFailure extends RuntimeException {
 final KeyImageError error;
 KeyImageFailure(KeyImageError e){super(e.message);error=e;}
 KeyImageFailure(KeyImageError e,String message){super(message);error=e;}
}
enum KeyImageError {
 GLOBAL_ERROR("KEY_IMG_GLOBAL_ERROR","Key image global error",400),
 MISSING_TAG("KEY_IMG_MISSING_TAG","Missing required DICOM tag",400),
 SUPPLEMENT_ERROR("KEY_IMG_SUPPLEMENT_ERROR","Error supplement value",400),
 SAVE_FILE_ERROR("KEY_IMG_SAVE_FILE_ERROR","Error saving key image file",500);
 final String code,message;final int status;
 KeyImageError(String c,String m,int s){code=c;message=m;status=s;}
}
