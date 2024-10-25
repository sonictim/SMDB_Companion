use cocoa::appkit::{NSBackingStoreType, NSView, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use eframe::NativeOptions;
use objc::{class, msg_send, sel, sel_impl};
use raw_window_handle::{AppKitWindowHandle, HasRawWindowHandle, RawWindowHandle};
use std::sync::{Arc, Mutex};

pub struct SharedState {
    windows: Vec<id>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
        }
    }

    pub fn add_window(&mut self, window: id) {
        self.windows.push(window);
    }
}

struct CocoaWindow {
    window: id,
    view: id,
}

// Implement raw window handle for our custom window
unsafe impl HasRawWindowHandle for CocoaWindow {
    fn raw_window_handle(&self) -> RawWindowHandle {
        let mut handle = AppKitWindowHandle::empty();
        handle.ns_window = self.window as *mut _;
        handle.ns_view = self.view as *mut _;
        RawWindowHandle::AppKit(handle)
    }
}

pub fn create_new_window(shared_state: Arc<Mutex<SharedState>>, title: &str, size: (f64, f64)) {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);

        // Create the window
        let window: id = msg_send![class!(NSWindow), alloc];
        let rect = NSRect::new(NSPoint::new(100.0, 100.0), NSSize::new(size.0, size.1));

        let style_mask = NSWindowStyleMask::NSTitledWindowMask
            | NSWindowStyleMask::NSClosableWindowMask
            | NSWindowStyleMask::NSResizableWindowMask
            | NSWindowStyleMask::NSMiniaturizableWindowMask;

        let window: id = msg_send![window, initWithContentRect:rect
                                              styleMask:style_mask
                                              backing:NSBackingStoreType::NSBackingStoreBuffered
                                              defer:NO];

        // Set the window title
        let ns_title = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![window, setTitle:ns_title];

        // Create and set up the view
        let view: id = msg_send![class!(NSView), alloc];
        let _: () = msg_send![view, initWithFrame:rect];
        let _: () = msg_send![window, setContentView:view];

        // Create our custom window wrapper
        let cocoa_window = CocoaWindow { window, view };

        // Set up eframe
        let native_options = NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([size.0 as f32, size.1 as f32])
                .with_min_inner_size([200.0, 200.0]),
            ..Default::default()
        };

        // Store the window in shared state
        shared_state.lock().unwrap().add_window(window);

        // Create the eframe runner
        let runner = eframe::Runner::new(
            native_options,
            Box::new(|cc| Box::new(crate::App::new_secondary_window(cc))),
        );

        // Initialize the runner with our window
        if let Ok(runner) = unsafe { runner.init_raw_window(cocoa_window) } {
            // Start the main event loop
            runner.run();
        }

        let _: () = msg_send![window, makeKeyAndOrderFront:nil];
        pool.drain();
    }
}
