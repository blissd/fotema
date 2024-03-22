use gtk::glib;
use gtk::prelude::{BoxExt, OrientableExt};
use photos_core;
use relm4::factory::{DynamicIndex, FactoryComponent, FactorySender, FactoryVecDeque};
use relm4::*;
use std::path;

#[derive(Debug)]
pub enum InputMsg {
    View,
}

#[derive(Debug)]
pub struct PicturePreview {
    path: path::PathBuf,
    //controller: photos_core::Controller,
    //pictures: Vec<photos_core::repo::Picture>,
}

#[relm4::factory(pub)]
impl FactoryComponent for PicturePreview {
    type Init = path::PathBuf;
    type Input = InputMsg;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
    #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            gtk::Picture {
                set_filename: Some(&self.path),
            }
        }
    }

    fn init_model(path: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { path }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {}
}

pub struct AllPhotos {
    //    controller: photos_core::Controller,
    pictures: FactoryVecDeque<PicturePreview>,
}

#[relm4::component(pub)]
impl SimpleComponent for AllPhotos {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            #[local_ref]
            pictures_box -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,
            }
        }
    }

    fn init(
        counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let repo = {
            let db_path = glib::user_data_dir().join("photos");
            let _ = std::fs::create_dir_all(&db_path);
            let db_path = db_path.join("pictures.sqlite");
            photos_core::Repository::open(&db_path).unwrap()
        };

        let scan = {
            let pic_dir = path::Path::new("/var/home/david/Pictures");
            photos_core::Scanner::build(&pic_dir).unwrap()
        };

        let mut controller = photos_core::Controller::new(repo, scan);
        let _ = controller.scan();
        let all_pictures = controller.all().unwrap();

        let mut pictures = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |_output| {});

        for p in all_pictures.iter().take(10) {
            let path = path::PathBuf::from("/var/home/david/Pictures").join(&p.relative_path);
            pictures.guard().push_back(path);
        }

        let model = AllPhotos { pictures };

        let pictures_box = model.pictures.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {}
}
