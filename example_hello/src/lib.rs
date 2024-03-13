use xilem::view::{button, h_stack, switch, v_stack};
use xilem::{view::View, App, AppLauncher};

#[cfg(target_os = "android")]
use xilem::winit::platform::android::activity::AndroidApp;

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    // here's some logic, deriving state for the view from our state;

    // The actual UI Code starts here
    switch::<AppData, _>(data.is_on, |data, is_on| data.is_on = is_on)
}

struct AppData {
    count: i32,
    is_on: bool,
}

#[allow(dead_code)]
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    let data = AppData {
        count: 0,
        is_on: false,
    };
    AppLauncher::new(App::new(data, app_logic), app).run()
}

#[allow(dead_code)]
#[cfg(not(target_os = "android"))]
pub fn main() {
    let data = AppData {
        count: 0,
        is_on: false,
    };
    AppLauncher::new(App::new(data, app_logic)).run()
}
