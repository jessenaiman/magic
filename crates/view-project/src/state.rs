#[derive(Clone, Debug, Default)]
pub(crate) struct HomeFormState {
    pub search: String,
    pub filter: String,
    pub show_new_form: bool,
    pub new_name: String,
    pub new_root: String,
    pub new_audio: String,
    pub wizard_style: String,
    pub wizard_world: String,
    pub wizard_character: String,
    pub wizard_motion: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectViewState {
    pub search: String,
    pub type_filter: String,
    pub group_by_folder: bool,
    pub link_popup_path: Option<String>,
    pub link_track: String,
}

impl Default for ProjectViewState {
    fn default() -> Self {
        Self { search: String::new(), type_filter: "all".into(), group_by_folder: true, link_popup_path: None, link_track: "V2".into() }
    }
}
