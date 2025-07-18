use objc2::rc::Retained;
use objc2::{declare_class, msg_send, mutability, ClassType, DeclaredClass};
use objc2_app_kit::{NSApplication, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol};

declare_class!(
    pub struct AppDelegate;

    unsafe impl ClassType for AppDelegate {
        type Super = NSObject;
        type Mutability = mutability::MainThreadOnly;
        const NAME: &'static str = "NotecognitoAppDelegate";
    }

    impl DeclaredClass for AppDelegate {
        type Ivars = ();
    }

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[method(applicationDidFinishLaunching:)]
        fn application_did_finish_launching(&self, _notification: &NSNotification) {
            tracing::info!("Application did finish launching");
        }

        #[method(applicationShouldTerminateAfterLastWindowClosed:)]
        fn application_should_terminate_after_last_window_closed(&self, _app: &NSApplication) -> bool {
            // Don't terminate when windows close (menu bar app)
            false
        }
    }

    // Custom methods
    unsafe impl AppDelegate {
        #[method(configure:)]
        fn configure(&self, _sender: &NSObject) {
            tracing::info!("Configure menu item clicked");
            crate::launch_config_ui();
        }

        #[method(about:)]
        fn about(&self, _sender: &NSObject) {
            tracing::info!("About menu item clicked");
            unsafe {
                let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
                msg_send![&app, orderFrontStandardAboutPanel: self];
            }
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send_id![mtm.alloc::<Self>(), init] }
    }
}