use native_windows_derive as nwd;
use native_windows_gui as nwg;

use anyhow::*;
use flume::{Receiver, Sender};
use nwd::NwgUi;
use nwg::{EmbedResource, Icon, NativeUi};
use systemstat::{Platform, System};

const MAX_CAT_INDEX: usize = 4;

pub struct Cat {
    pub dark: Vec<Icon>,
    pub light: Vec<Icon>,
}

#[derive(NwgUi)]
pub struct SystemTray {
    #[nwg_control]
    window: nwg::MessageWindow,

    #[nwg_resource(source_file: Some("./cat/dark_cat_0.ico"))]
    icon: nwg::Icon,

    #[nwg_control]
    #[nwg_events( OnNotice: [SystemTray::update] )]
    notice: nwg::Notice,

    #[nwg_control(icon: Some(&data.icon), tip: Some("ฅ(^•ω•^ฅ ◞ ̑̑"))]
    #[nwg_events(MousePressLeftUp: [SystemTray::show_menu], OnContextMenu: [SystemTray::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Theme")]
    tray_theme_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::exit])]
    tray_item_exit: nwg::MenuItem,

    #[nwg_control(parent: tray_theme_menu, text: "dark")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::change_to_dark])]
    tray_item_theme_dark: nwg::MenuItem,

    #[nwg_control(parent: tray_theme_menu, text: "light")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::change_to_light])]
    tray_item_theme_light: nwg::MenuItem,

    resource: Cat,

    sender: Sender<usize>,
    receiver: Receiver<(usize, bool)>,
}

impl SystemTray {
    fn new(sender: Sender<usize>, receiver: Receiver<(usize, bool)>) -> SystemTray {
        let embed = nwg::EmbedResource::load(None).unwrap();
        let cat = Cat::load(embed);

        SystemTray {
            window: Default::default(),
            icon: Default::default(),
            notice: Default::default(),
            tray: Default::default(),
            tray_menu: Default::default(),
            tray_theme_menu: Default::default(),
            tray_item_theme_dark: Default::default(),
            tray_item_theme_light: Default::default(),
            tray_item_exit: Default::default(),
            resource: cat,
            sender,
            receiver,
        }
    }

    fn update(&self) {
        if let Ok((index, is_dark)) = self.receiver.try_recv() {
            //let ico = Icon::from_embed(&self.embed, None, Some(&ico_name)).unwrap();
            //let ico = self.embed.icon_str(&ico_name, None).unwrap();
            let ico = if is_dark {
                &self.resource.dark[index]
            } else {
                &self.resource.light[index]
            };

            (&self).tray.set_icon(ico);
        }
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn change_to_dark(&self) {
        self.tray_item_theme_dark.set_checked(true);
        self.tray_item_theme_dark.set_enabled(false);
        self.tray_item_theme_light.set_checked(false);
        self.tray_item_theme_light.set_enabled(true);
        self.sender.send(1).unwrap();
    }

    fn change_to_light(&self) {
        self.tray_item_theme_dark.set_checked(false);
        self.tray_item_theme_dark.set_enabled(true);
        self.tray_item_theme_light.set_checked(true);
        self.tray_item_theme_light.set_enabled(false);
        self.sender.send(0).unwrap();
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

impl Cat {
    fn load(embed: EmbedResource) -> Cat {
        Cat {
            light: vec![
                embed.icon_str("DARK_CAT_0", None).unwrap(),
                embed.icon_str("DARK_CAT_1", None).unwrap(),
                embed.icon_str("DARK_CAT_2", None).unwrap(),
                embed.icon_str("DARK_CAT_3", None).unwrap(),
                embed.icon_str("DARK_CAT_4", None).unwrap(),
            ],
            dark: vec![
                embed.icon_str("LIGHT_CAT_0", None).unwrap(),
                embed.icon_str("LIGHT_CAT_1", None).unwrap(),
                embed.icon_str("LIGHT_CAT_2", None).unwrap(),
                embed.icon_str("LIGHT_CAT_3", None).unwrap(),
                embed.icon_str("LIGHT_CAT_4", None).unwrap(),
            ],
        }
    }
}

fn main() -> Result<()> {
    let sys = System::new();
    let (main_tx, sub_rx) = flume::unbounded();
    let (sub_tx, main_rx) = flume::unbounded();
    let (cpu_tx, cpu_rx) = flume::unbounded();

    nwg::init()?;
    let ui = SystemTray::build_ui(SystemTray::new(sub_tx, sub_rx))?;
    let notifier = ui.notice.sender();

    ui.tray_item_theme_dark.set_checked(false);
    ui.tray_item_theme_dark.set_enabled(true);
    ui.tray_item_theme_light.set_checked(true);
    ui.tray_item_theme_light.set_enabled(false);

    let mut i = 0;
    let mut is_dark = false;
    let mut usage_cache = 1.0;


    // cpu stats calculation thread
    std::thread::spawn(move || loop {
        let cpu_usage = if let Ok(cpu) = sys.cpu_load_aggregate() {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            let cpu = cpu.done().unwrap();
            100.0 - cpu.idle * 100.0
        } else {
            1.0
        };

        cpu_tx.send(cpu_usage).unwrap();
    });

    // update cat ico thread
    std::thread::spawn(move || loop {
        let cpu_usage = if let Ok(usage) = cpu_rx.try_recv() {
            usage_cache = usage;
            usage
        } else {
            usage_cache
        };
        if let Ok(index) = main_rx.try_recv() {
            is_dark = if index == 0 { false } else { true };
        }
        if i > MAX_CAT_INDEX {
            i = 0;
        }
        let cmp_f = [20.0, cpu_usage / 5.0];
        let min = cmp_f.iter().fold(0.0 / 0.0, |m, v| v.min(m));
        let cmp_f = [1.0, min];
        let max = cmp_f.iter().fold(0.0 / 0.0, |m, v| v.max(m));
        std::thread::sleep(std::time::Duration::from_millis((200.0 / max) as u64));
        main_tx.send((i, is_dark)).unwrap();
        notifier.notice();
        i += 1;
    });

    nwg::dispatch_thread_events();

    Ok(())
}
