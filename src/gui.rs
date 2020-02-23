use crate::{Gui, UINodeHandle};
use rg3d::{
    engine::resource_manager::ResourceManager,
    utils,
    resource::texture::TextureKind,
    gui::{
        scroll_bar::ScrollBarBuilder,
        check_box::CheckBoxBuilder,
        VerticalAlignment,
        HorizontalAlignment,
        widget::WidgetBuilder,
        image::ImageBuilder,
        Thickness,
        scroll_bar::Orientation,
        scroll_viewer::ScrollViewerBuilder,
        Control,
        widget::Widget,
        node::UINode
    }
};

#[derive(Debug)]
pub struct CustomUiMessage {}

#[derive(Debug)]
pub enum DummyUiNode {}

impl Control<CustomUiMessage, DummyUiNode> for DummyUiNode {
    fn widget(&self) -> &Widget<CustomUiMessage, DummyUiNode> {
        unimplemented!()
    }

    fn widget_mut(&mut self) -> &mut Widget<CustomUiMessage, DummyUiNode> {
        unimplemented!()
    }

    fn raw_copy(&self) -> UINode<CustomUiMessage, DummyUiNode> {
        unimplemented!()
    }
}

pub fn create_scroll_bar(ui: &mut Gui, resource_manager: &mut ResourceManager, orientation: Orientation) -> UINodeHandle {
    let mut wb = WidgetBuilder::new();
    match orientation {
        Orientation::Vertical => wb = wb.with_width(30.0),
        Orientation::Horizontal => wb = wb.with_height(30.0),
    }
    ScrollBarBuilder::new(wb)
        .with_orientation(orientation)
        .show_value(true)
        .with_indicator(ImageBuilder::new(WidgetBuilder::new())
            .with_opt_texture(utils::into_any_arc(resource_manager.request_texture("data/ui/circle.png", TextureKind::RGBA8)))
            .build(ui))
        .build(ui)
}

pub fn create_check_box(ui: &mut Gui, resource_manager: &mut ResourceManager) -> UINodeHandle {
    CheckBoxBuilder::new(WidgetBuilder::new()
        .with_margin(Thickness::uniform(2.0))
        .with_width(24.0)
        .with_height(24.0)
        .with_vertical_alignment(VerticalAlignment::Center)
        .with_horizontal_alignment(HorizontalAlignment::Left))
        .with_check_mark(ImageBuilder::new(WidgetBuilder::new())
            .with_opt_texture(utils::into_any_arc(resource_manager.request_texture("data/ui/check_mark.png", TextureKind::RGBA8)))
            .build(ui))
        .build(ui)
}

pub fn create_scroll_viewer(ui: &mut Gui, resource_manager: &mut ResourceManager) -> UINodeHandle {
    ScrollViewerBuilder::new(WidgetBuilder::new())
        .with_horizontal_scroll_bar(create_scroll_bar(ui, resource_manager, Orientation::Horizontal))
        .with_vertical_scroll_bar(create_scroll_bar(ui, resource_manager, Orientation::Vertical))
        .build(ui)
}