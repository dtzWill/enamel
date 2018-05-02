use std::sync::{Once, ONCE_INIT};
use std::cell::{Cell, RefCell};
use std::ptr;
use std::mem;
use std::ffi::CString;

use gio;
use glib;
use gtk;
use gdk;
use cairo;
use glib::translate::*;
use gtk::prelude::*;

use pango;
use pango::prelude::*;
use pango::LayoutExt;
use glib::object::Downcast;
use glib::value::AnyValue;
use notmuch;


use inox_core::settings::Settings;
use inox_core::database::Manager as DBManager;


use notmuch::DatabaseMode;

use gobject_subclass::object::*;
use gobject_subclass::properties::*;

use cell_renderer::*;


pub trait CellRendererThreadImpl: 'static {

}

pub struct CellRendererThreadSettings {
    // cached values
    height_set: bool,
    left_icons_size: i32,
    left_icons_width: i32,

    left_icons_width_n: i32,
    left_icons_padding: i32,
    date_start: i32,
    date_len: i32,
    date_width: i32,

    message_count_start: i32,
    message_count_len: i32,
    message_count_width: i32,

    authors_start: i32,
    authors_len: i32,
    authors_width: i32,

    tags_start: i32,
    tags_width: i32,
    tags_len: i32,

    subject_start: i32,
    height: i32,

    // configurable values
    font_description: pango::FontDescription,
    language: pango::Language,
    content_height: i32,
    line_spacing : i32,
    date_length : i32,
    message_count_length : i32,
    authors_length : i32,

    subject_color: String,
    subject_color_selected : String,
    background_color_selected : String,
    background_color_marked : String,
    background_color_marked_selected : String,

    tags_length : u16,
    tags_upper_color : String,
    tags_lower_color : String,
    tags_alpha : String,
    hidden_tags : Vec<String>
}

impl Default for CellRendererThreadSettings{

    fn default() -> Self{

        CellRendererThreadSettings
        {
            height_set: false,
            content_height: 0,
            left_icons_size: 0,
            left_icons_width: 0,

            left_icons_width_n: 2,
            left_icons_padding: 1,
            date_start: 0,
            date_len: 10,
            date_width: 0,

            message_count_start: 0,
            message_count_len: 4,
            message_count_width: 0,

            authors_start: 0,
            authors_len: 20,
            authors_width: 0,

            tags_start: 0,
            tags_width: 0,
            tags_len: 80,

            subject_start: 0,
            height: 0,

            language: pango::Language::default(),
            font_description : pango::FontDescription::from_string("default"),
            line_spacing : 2,
            date_length : 10,
            message_count_length : 4,
            authors_length : 20,

            subject_color : "#807d74".to_string(),
            subject_color_selected : "#000000".to_string(),
            background_color_selected : "".to_string(),
            background_color_marked : "#fff584".to_string(),
            background_color_marked_selected : "#bcb559".to_string(),

            tags_length : 80,
            tags_upper_color : "#e5e5e5".to_string(),
            tags_lower_color : "#333333".to_string(),
            tags_alpha : "0.5".to_string(),
            hidden_tags : ["attachment".to_string(),
                           "flagged".to_string(),
                           "unread".to_string()].to_vec(),
        }
    }
}










pub struct CellRendererThread {
    thread: RefCell<Option<notmuch::Thread>>,
    settings: RefCell<CellRendererThreadSettings>
}
//
fn threadfun() -> glib::Type{

    AnyValue::static_type()
}

static PROPERTIES: [Property; 1] = [
    Property::Boxed(
        "thread",
        "Thread to display",
        "Handle of notmuch::Thread to display",
        threadfun,
        PropertyMutability::ReadWrite,
    ),


];

impl CellRendererThread {
    pub fn new() -> CellRenderer {
        use glib::object::Downcast;

        static ONCE: Once = ONCE_INIT;
        static mut TYPE: glib::Type = glib::Type::Invalid;

        ONCE.call_once(|| {
            let static_instance = CellRendererThreadStatic::default();
            let t = register_type(static_instance);
            unsafe {
                TYPE = t;
            }
        });

        unsafe {
            glib::Object::new(TYPE, &[]).unwrap().downcast_unchecked()
        }
    }

    fn class_init(klass: &mut CellRendererClass) {
        klass.install_properties(&PROPERTIES);
    }

    fn init(renderer: &CellRenderer) -> Box<CellRendererImpl<CellRenderer>>
    {
        let imp = Self{
            thread: RefCell::new(None),
            settings: RefCell::new(CellRendererThreadSettings::default())
        };
        Box::new(imp)
    }

    fn calculate_height(&self, widget: &gtk::Widget) -> i32
    {
        let settings = self.settings.borrow();
        if settings.height_set {
            return settings.content_height;
        }

       /* figure out font height */
       let pango_layout = widget.create_pango_layout("TEST HEIGHT STRING").unwrap();

       pango_layout.set_font_description(&settings.font_description);

       let (w, h) = pango_layout.get_pixel_size();

       self.settings.borrow_mut().content_height = h;

       return h;
   }
}



impl ObjectImpl<CellRenderer> for CellRendererThread{


    fn set_property(&self, obj: &glib::Object, id: u32, value: &glib::Value) {
        let prop = &PROPERTIES[id as usize];

        match *prop {
            Property::Boxed("thread", ..) => {
                let any_v = value.get::<&AnyValue>().expect("Value did not actually contain an AnyValue");
                *(self.thread.borrow_mut()) = Some(any_v.downcast_ref::<notmuch::Thread>().unwrap().clone());
            },
            _ => unimplemented!(),
        }
    }

    fn get_property(&self, obj: &glib::Object, id: u32) -> Result<glib::Value, ()> {
        let prop = &PROPERTIES[id as usize];

        match *prop {
            Property::Boxed("thread", ..) => {
                Ok("1".to_value())
            },
            _ => unimplemented!(),
        }
    }
}

impl CellRendererImpl<CellRenderer> for CellRendererThread {

    fn render(&self,
        renderer: &CellRenderer,
        cr: &cairo::Context,
        widget: &gtk::Widget,
        _background_area: &gtk::Rectangle,
        cell_area: &gtk::Rectangle,
        _flags: gtk::CellRendererState,
    ){

        let settings = self.settings.borrow();

        // calculate text width, we don't need to do this every time,
        // but we need access to the context.
        if  !settings.height_set {
            let pango_cr = widget.create_pango_context().unwrap();
            let font_metrics = pango_cr.get_metrics(&settings.font_description, &settings.language).unwrap();

            let char_width = font_metrics.get_approximate_char_width() / pango::SCALE;
            let padding = char_width;

            let content_height  = self.calculate_height(widget);

            let mut settings_mut = self.settings.borrow_mut();

            let line_height = content_height + settings.line_spacing;

            settings_mut.height_set = true;

            settings_mut.left_icons_size  = content_height - (2 * settings.left_icons_padding);
            settings_mut.left_icons_width = settings.left_icons_size;

            settings_mut.date_start          = settings.left_icons_width_n * settings.left_icons_width +
                  (settings.left_icons_width_n-1) * settings.left_icons_padding + padding;
            settings_mut.date_width          = char_width * settings.date_len;
            settings_mut.message_count_width = char_width * settings.message_count_len;
            settings_mut.message_count_start = settings.date_start + settings.date_width + padding;
            settings_mut.authors_width       = char_width * settings.authors_len;
            settings_mut.authors_start       = settings.message_count_start + settings.message_count_width + padding;
            settings_mut.tags_width          = char_width * settings.tags_len;
            settings_mut.tags_start          = settings.authors_start + settings.authors_width + padding;
            settings_mut.subject_start       = settings.tags_start + settings.tags_width + padding;

            settings_mut.height              = content_height + settings.line_spacing;
        }

        if self.thread.borrow().is_none(){
            return;
        }

        let thread = self.thread.borrow().as_ref().unwrap().clone();

        /*if thread.unread() {
          self.settings.borrow_mut().font_description.set_weight(pango::Weight::Bold);
        } else */ {
          self.settings.borrow_mut().font_description.set_weight(pango::Weight::Normal);
        }

        // render_background (cr, widget, background_area, flags);
        // render_date (cr, widget, cell_area); // returns height

        if thread.total_messages() > 1 {
          //render_message_count (cr, widget, cell_area);
        }

        // render_authors (cr, widget, cell_area);
        //
        // tags_width = render_tags (cr, widget, cell_area, flags); // returns width
        // subject_start = tags_start + tags_width / Pango::SCALE + ((tags_width > 0) ? padding : 0);
        //
        // render_subject (cr, widget, cell_area, flags);


        // if (thread->flagged)
        //   render_flagged (cr, widget, cell_area);
        //
        // if (thread->attachment)
        //   render_attachment (cr, widget, cell_area);
        //
        // /*
        // if (marked)
        //   render_marked (cr, widget, cell_area);
        // */

    }

}

#[derive(Default)]
pub struct CellRendererThreadStatic{
}

impl ImplTypeStatic<CellRenderer> for CellRendererThreadStatic {
    fn get_name(&self) -> &str {
        "CellRendererThread"
    }

    fn new(&self, renderer: &CellRenderer) -> Box<CellRendererImpl<CellRenderer>> {
        CellRendererThread::init(renderer)
    }

    fn class_init(&self, klass: &mut CellRendererClass) {
        CellRendererThread::class_init(klass);
    }

    fn type_init(&self, _token: &TypeInitToken, _type: glib::Type) {

    }
}
