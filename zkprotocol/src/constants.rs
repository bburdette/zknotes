pub enum Error {
  NotFound,
}

pub enum SpecialUuids {
  Public,
  Comment,
  Share,
  Search,
  User,
  Archive,
  System,
  Sync,
}

impl SpecialUuids {
  pub fn str(&self) -> &str {
    match *self {
      SpecialUuids::Public => "f596bc2c-a882-4c1c-b739-8c4e25f34eb2",
      SpecialUuids::Comment => "e82fefee-bcd3-4e2e-b350-9963863e516d",
      SpecialUuids::Share => "466d39ec-2ea7-4d43-b44c-1d3d083f8a9d",
      SpecialUuids::Search => "84f72fd0-8836-43a3-ac66-89e0ab49dd87",
      SpecialUuids::User => "4fb37d76-6fc8-4869-8ee4-8e05fa5077f7",
      SpecialUuids::Archive => "ad6a4ca8-0446-4ecc-b047-46282ced0d84",
      SpecialUuids::System => "0efcc98f-dffd-40e5-af07-90da26b1d469",
      SpecialUuids::Sync => "528ccfc2-8488-41e0-a4e1-cbab6406674e",
    }
  }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum PrivateRequests {
  GetZkNote,
  GetZkNoteAndLinks,
  GetZnlIfChanged,
  GetZkNoteComments,
  GetZkNoteArchives,
  GetArchiveZkNote,
  GetArchiveZklinks,
  GetZkLinksSince,
  SearchZkNotes,
  PowerDelete,
  DeleteZkNote,
  SaveZkNote,
  SaveZkLinks,
  SaveZkNoteAndLinks,
  SaveImportZkNotes,
  SetHomeNote,
  SyncRemote,
  SyncFiles,
  GetJobStatus,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum PrivateStreamingRequests {
  SearchZkNotes,
  GetArchiveZkLinks,
  GetZkLinksSince,
  Sync,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum PublicRequests {
  GetZkNoteAndLinks,
  GetZnlIfChanged,
  GetZkNotePubId,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum PublicReplies {
  ServerError,
  ZkNoteAndLinks,
  Noop,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum PrivateReplies {
  ServerError,
  ZkNote,
  ZkNoteAndLinksWhat,
  Noop,
  ZkNoteComments,
  Archives,
  ZkNoteArchives,
  ArchiveZkLinks,
  ZkLinks,
  ZkListNoteSearchResult,
  ZkNoteSearchResult,
  ZkNoteIdSearchResult,
  ZkNoteAndLinksSearchResult,
  PowerDeleteComplete,
  DeletedZkNote,
  SavedZkNote,
  SavedZkLinks,
  SavedZkNoteAndLinks,
  SavedImportZkNotes,
  HomeNoteSet,
  JobStatus,
  JobNotFound,
  SyncComplete,
  FileSyncComplete,
  NotLoggedIn,
  LoginError,
  FilesUploaded,
}
