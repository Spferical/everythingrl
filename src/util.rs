use std::future::Future;

pub fn trim(text: &str, max_chars: usize) -> &str {
    if text.chars().next().is_none() {
        return "";
    }
    let length = text.len().min(max_chars);
    let mut iter = text.char_indices();
    let (end, _) = iter
        .nth(length)
        .unwrap_or(text.char_indices().last().unwrap());
    &text[..end]
}

#[cfg(target_family = "wasm")]
pub fn spawn(future: impl Future<Output = ()> + 'static) {
    wasm_bindgen_futures::spawn_local(future);
}
#[cfg(not(target_family = "wasm"))]
pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
    smol::spawn(async_compat::Compat::new(future)).detach();
}

#[cfg(not(target_family = "wasm"))]
pub fn download_file(file_name: String, content: String) {
    spawn(async move {
        if let Some(handle) = rfd::AsyncFileDialog::new()
            .set_file_name(file_name)
            .save_file()
            .await
        {
            handle.write(content.as_bytes()).await.ok();
        }
    })
}

// rfd does not support saving files from wasm.
#[cfg(target_family = "wasm")]
pub fn download_file(file_name: String, content: String) {
    use wasm_bindgen::prelude::*;
    use web_sys::{window, Blob, HtmlAnchorElement, Url};

    match (|| -> Result<(), JsValue> {
        // Create a Blob from the content
        let blob = Blob::new_with_str_sequence(&js_sys::Array::of1(&content.into()))?;

        // Create an Object URL
        let url = Url::create_object_url_with_blob(&blob)?;

        // Create a temporary <a> element
        let document = window().unwrap().document().unwrap();
        let a = document
            .create_element("a")?
            .dyn_into::<HtmlAnchorElement>()?;
        a.set_href(&url);
        a.set_download(&file_name);

        // Trigger a click event
        a.click();

        // Revoke the Object URL
        Url::revoke_object_url(&url)?;
        Ok(())
    })() {
        Err(js_err) => {
            macroquad::miniquad::error!("Error saving file: {:?}", js_err);
        }
        Ok(()) => {}
    }
}
