# drag-rs

Start a drag operation out of a window on macOS, Windows and Linux (via GTK).

Tested for [tao](https://github.com/tauri-apps/tao) (latest), [winit](https://github.com/rust-windowing/winit) (latest), [wry](https://github.com/tauri-apps/wry) (v0.24) and [tauri](https://github.com/tauri-apps/tauri) (v1) windows.
Due to the GTK-based implementation, winit currently cannot leverage this crate on Linux yet.

This project also includes a Tauri plugin for simplified usage on Tauri apps.

## Setup

There's two ways to consume this crate API: from Rust code via the `drag` crate or from Tauri's frontend via `tauri-plugin-drag`.

### Rust

- Add the `drag` dependency:

`$ cargo add drag`

- Define the drag item and preview icon:

  ```rust
  let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
  let preview_icon = drag::Image::Raw(include_bytes!("../../icon.png").to_vec());
  ```

- Use the `drag::start_drag` function. It takes a `&T: raw_window_handle::HasRawWindowHandle` type on macOS and Windows, and a `&gtk::ApplicationWindow` on Linux:

  - tao:
  ```rust
  let event_loop = tao::event_loop::EventLoop::new();
  let window = tao::window::WindowBuilder::new().build(&event_loop).unwrap();

  drag::start_drag(
    #[cfg(target_os = "linux")]
    {
      use tao::platform::unix::WindowExtUnix;
      window.gtk_window()
    },
    #[cfg(not(target_os = "linux"))]
    &window,
    item,
    preview_icon,
  );
  ```

  - wry:
  ```rust
  let event_loop = wry::application::event_loop::EventLoop::new();
  let window = wry::application::window::WindowBuilder::new().build(&event_loop).unwrap();
  let webview = wry::webview::WebViewBuilder::new(window).unwrap().build().unwrap();

  drag::start_drag(
    #[cfg(target_os = "linux")]
    {
      use wry::application::platform::unix::WindowExtUnix;
      webview.window().gtk_window()
    },
    #[cfg(not(target_os = "linux"))]
    &webview.window(),
    item,
    preview_icon,
  );
  ```

  - winit:
  ```rust
  let event_loop = winit::event_loop::EventLoop::new().unwrap();
  let window = winit::window::WindowBuilder::new().build(&event_loop).unwrap();
  let _ = drag::start_drag(&window, item, preview_icon);
  ```

  - tauri:
  ```rust
  tauri::Builder::default()
    .setup(|app| {
      let window = app.get_window("main").unwrap();

      drag::start_drag(
        #[cfg(target_os = "linux")]
        &window.gtk_window()?,
        #[cfg(not(target_os = "linux"))]
        &window,
        item,
        preview_icon
      );

      Ok(())
    })
  ```

### Tauri Plugin

- Add the `tauri-plugin-drag` dependency:

`$ cargo add tauri-plugin-drag`

- Install the `@crabnebula/tauri-plugin-drag` NPM package containing the API bindings:

```sh
pnpm add @crabnebula/tauri-plugin-drag
# or
npm add @crabnebula/tauri-plugin-drag
# or
yarn add @crabnebula/tauri-plugin-drag
```

- Register the core plugin with Tauri:

`src-tauri/src/main.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- Afterwards all the plugin's APIs are available through the JavaScript guest bindings:

```javascript
import { startDrag } from "@crabnebula/tauri-plugin-drag";
startDrag({ item: ['/path/to/drag/file'], icon: '/path/to/icon/image' })
```

## Examples

Running the examples:

```sh
cargo run --bin [tauri-app|winit-app|tao-app|wry-app]
```

Additional drag as window examples are available for tauri and wry:

```sh
cargo run --bin [tauri-app-dragout|wry-app-dragout]
```

## Licenses

MIT or MIT/Apache 2.0 where applicable.
