use sapp_jsutils::JsObject;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::sync::mpsc::Sender;

extern "C" {
    pub fn req_post(ptr: *const i8, len: u32) -> u32;
}

thread_local! {
    static SENDERS: RefCell<HashMap<u32, Sender<Result<String, String>>>> = RefCell::new(HashMap::new());
}

#[derive(serde::Serialize)]
struct PostArgs {
    url: String,
    json_payload: String,
}

pub fn post(url: String, json_payload: String, tx: Sender<Result<String, String>>) {
    let args = PostArgs { url, json_payload };
    let encoded = serde_json::to_string(&args).unwrap();
    let encoded = CString::new(encoded.as_bytes()).unwrap();
    let id = unsafe { req_post(encoded.as_ptr(), encoded.as_bytes().len() as u32) };
    SENDERS.with(|senders| {
        let mut senders = senders.borrow_mut();
        senders.insert(id, tx);
    });
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
enum PostResult {
    Success(String),
    Error(String),
}

#[no_mangle]
pub extern "C" fn request_done(file_id: u32, result: JsObject) {
    let mut resp = String::new();
    result.to_string(&mut resp);
    macroquad::miniquad::debug!("{}", resp);
    let resp: PostResult = serde_json::from_str(&resp.trim()).unwrap();
    SENDERS.with(|senders| {
        let mut senders = senders.borrow_mut();
        if let Some(sender) = senders.remove(&file_id) {
            let _ = sender.send(match resp {
                PostResult::Success(s) => Ok(s),
                PostResult::Error(s) => Err(s),
            });
        };
    })
}
