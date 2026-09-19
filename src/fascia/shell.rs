// src/fascia/shell.rs
use	crate::fascia::theme::{ FasciaStyle, ThemePalette };
use	iced::widget::{ column, container, row };
use	iced::{ Element, Length };
/// Builds the complete shell layout containing Menubar, Toolbar, Activity Bar,
/// Sidebar, Content Area, and Status Bar.
pub fn	view_shell< 'a, Message: 'static>( 
    menubar: Element< 'a, Message>,
    toolbar: Option< Element< 'a, Message>>,
    activity_bar: Element< 'a, Message>,
    sidebar: Option< Element< 'a, Message>>,
    content: Element< 'a, Message>,
    status_bar: Option< Element< 'a, Message>>,
    palette: ThemePalette,
) -> Element< 'a, Message> {
    let  	mut middle_row = row![activity_bar].height( Length::Fill);
    if let  	Some( sb) = sidebar {
        middle_row = middle_row.push( sb);
    }
    middle_row = middle_row.push( 
        container( content)
            .width( Length::Fill)
            .height( Length::Fill)
            .style( move |_| FasciaStyle::content_container( palette)),
    );
    let  	mut layout = column![menubar];
    if let  	Some( tb) = toolbar {
        layout = layout.push( tb);
    }
    layout = layout.push( middle_row);
    if let  	Some( sb) = status_bar {
        layout = layout.push( sb);
    }
    container( layout)
        .width( Length::Fill)
        .height( Length::Fill)
        .into()
}
