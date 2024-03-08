use sapp_jsutils::JsObject;
extern "C" {
    pub fn req_make_request(js_object: JsObject) -> u32;
}

pub fn post(url: String, json_payload: String) {
    let mut js_obj = JsObject::object();
    js_obj.set_field_string("url", &url);
    js_obj.set_field_string("payload", &json_payload);
    let id = req_make_request(js_obj);
}

#[no_mangle]
pub extern "C" fn request_done(file_id: u32, result: JsObject) {
    eprintln!("{} {:?}", file_id, result);
    todo!()
}
