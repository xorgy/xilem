use xilem::view::{button, h_stack, switch, v_stack};
use xilem::{view::View, App, AppLauncher};

#[cfg(target_os = "android")]
use xilem::winit::platform::android::activity::AndroidApp;

fn app_logic(data: &mut AppData) -> impl View<AppData> {
    // here's some logic, deriving state for the view from our state
    let count = data.count;
    let label = if count == 1 {
        "clicked 1 time".to_string()
    } else {
        format!("clicked {count} times")
    };

    // The actual UI Code starts here
    v_stack((
        button(label, |data: &mut AppData| {
            println!("clicked");
            data.count += 1;
        }),
        h_stack((
            button("decrease", |data: &mut AppData| {
                println!("clicked decrease");
                data.count -= 1;
            }),
            button("reset", |data: &mut AppData| {
                println!("clicked reset");
                data.count = 0;
            }),
            switch(data.is_on, |data: &mut AppData, value: bool| {
                data.is_on = value;
            }),
        )),
    ))
    .with_spacing(20.0)
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
fn main() {
    let data = AppData {
        count: 0,
        is_on: false,
    };
    AppLauncher::new(App::new(data, app_logic)).run()
}
