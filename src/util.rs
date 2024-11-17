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
