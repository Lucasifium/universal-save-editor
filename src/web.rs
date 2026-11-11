//! The browser version: started by `web/index.html`, it runs entirely on the visitor's
//! computer. Files come in through the browser's file picker or by dropping them on the page,
//! and go back out as downloads; nothing is uploaded anywhere.

use eframe::egui;
use std::cell::RefCell;
use std::path::PathBuf;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

thread_local! {
    /// Files the picker has finished reading, waiting for the app to take them.
    static PICKED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
}

/// Starts the app in the `<canvas>` with this id.
#[wasm_bindgen]
pub async fn start(canvas_id: String) -> Result<(), JsValue> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&canvas_id))
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(crate::app::App::new(cc, None)))),
        )
        .await
}

/// Keeps a file the user gave the page, under its own name, and returns where it is.
pub fn keep(name: &str, data: &[u8]) -> PathBuf {
    let name = name
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or("save");
    let p = PathBuf::from(name);
    let _ = crate::vfs::write(&p, data);
    p
}

/// Opens the browser's file picker; the chosen file arrives later through `take_picked`.
pub fn pick_file(ctx: &egui::Context) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(input) = doc.create_element("input").and_then(|e| {
        e.dyn_into::<web_sys::HtmlInputElement>()
            .map_err(Into::into)
    }) else {
        return;
    };
    input.set_type("file");
    let ctx = ctx.clone();
    let inp = input.clone();
    let on_change = Closure::once_into_js(move || {
        let Some(file) = inp.files().and_then(|f| f.get(0)) else {
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(buf) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await else {
                return;
            };
            let data = js_sys::Uint8Array::new(&buf).to_vec();
            let p = keep(&file.name(), &data);
            PICKED.with(|q| q.borrow_mut().push(p));
            ctx.request_repaint();
        });
    });
    input.set_onchange(Some(on_change.unchecked_ref()));
    input.click();
}

/// The next file the picker delivered, if any.
pub fn take_picked() -> Option<PathBuf> {
    PICKED.with(|q| {
        let mut q = q.borrow_mut();
        (!q.is_empty()).then(|| q.remove(0))
    })
}

/// Hands `data` to the browser as a download named `name`.
pub fn download(name: &str, data: &[u8]) -> Result<(), String> {
    let err = |e: JsValue| e.as_string().unwrap_or_else(|| format!("{e:?}"));
    let doc = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(data));
    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/octet-stream");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).map_err(err)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(err)?;
    let a = doc
        .create_element("a")
        .map_err(err)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "no link")?;
    a.set_href(&url);
    a.set_download(name);
    a.click();
    // Let the browser start the download before the address is let go.
    let revoke = Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    if let Some(w) = web_sys::window() {
        let _ =
            w.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 10_000);
    }
    Ok(())
}
