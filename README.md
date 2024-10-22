# drag-rs

Start a drag operation out of a window on macOS, Windows and Linux (via GTK).

Tested for [tao](https://github.com/tauri-apps/tao) (latest), [winit](https://github.com/rust-windowing/winit) (latest), [wry](https://github.com/tauri-apps/wry) (v0.46) and [tauri](https://github.com/tauri-apps/tauri) (v2) windows.
Due to the GTK-based implementation, winit currently cannot leverage this crate on Linux yet.

This project also includes a Tauri plugin for simplified usage on Tauri apps.

## Setup

There's two ways to consume this crate API: from Rust code via the `drag` crate or from Tauri's frontend via `tauri-plugin-drag` or `tauri-plugin-drag-as-window`.

### Rust

- Add the `drag` dependency:

`$ cargo add drag`

- Define the drag item and preview icon:

  ```rust
  let item = drag::DragItem::Files(vec![std::fs::canonicalize("./examples/icon.png").unwrap()]);
  let preview_icon = drag::Image::Raw(include_bytes!("../../icon.png").to_vec());
  ```

- Use the `drag::start_drag` function. It takes a `&T: raw_window_handle::HasWindowHandle` type on macOS and Windows, and a `&gtk::ApplicationWindow` on Linux:

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
  let event_loop = tao::event_loop::EventLoop::new();
  let window = tao::window::WindowBuilder::new().build(&event_loop).unwrap();
  let webview = wry::WebViewBuilder::new().build(&window).unwrap();

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

#### tauri-plugin-drag

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

- Add permissions

`capabilities/default.json`

```json
{
    ...
    "permissions": [
        ...
        "drag:default"
    ]
}
```

- Afterwards all the plugin's APIs are available through the JavaScript guest bindings:

```javascript
import { startDrag } from "@crabnebula/tauri-plugin-drag";
startDrag({ item: ['/path/to/drag/file'], icon: '/path/to/icon/image' })
```

#### tauri-plugin-drag-as-window

- Add the `tauri-plugin-drag-as-window` dependency:

`$ cargo add tauri-plugin-drag-as-window`

- Install the `@crabnebula/tauri-plugin-drag-as-window` NPM package containing the API bindings:

```sh
pnpm add @crabnebula/tauri-plugin-drag-as-window
# or
npm add @crabnebula/tauri-plugin-drag-as-window
# or
yarn add @crabnebula/tauri-plugin-drag-as-window
```

- Register the core plugin with Tauri:

`src-tauri/src/main.rs`

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_drag_as_window::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- Add permissions

`capabilities/default.json`

```json
{
    ...
    "permissions": [
        ...
        "drag-as-window:default"
    ]
}
```

- Afterwards all the plugin's APIs are available through the JavaScript guest bindings:

```javascript
import { dragAsWindow, dragBack } from "@crabnebula/tauri-plugin-drag-as-window";
import { getCurrentWebviewWindow, WebviewWindow } from "@tauri-apps/api/webviewWindow";
// alternatively you can pass a DOM element instead of its selector
dragAsWindow('#my-drag-element', (payload) => {
  console.log('dropped!')
  // create the window with the content from the current element (that's is up to you!)
  new WebviewWindow('label', {
    x: payload.cursorPos.x,
    y: payload.cursorPos.y,
  })
})

const el = document.querySelector('#my-drag-element')
el.ondragstart = (event) => {
  event.preventDefault()

  dragBack(event.target, { data: 'some data' }, (payload) => {
    getCurrentWebviewWindow().close()
  })
}
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
