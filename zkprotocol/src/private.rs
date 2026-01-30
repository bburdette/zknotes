use crate::{
  content::{
    ArchiveZkLink, GetArchiveZkLinks, GetZkLinksSince, GetZkNoteAndLinks, GetZkNoteArchives,
    GetZkNoteComments, GetZnlIfChanged, ImportZkNote, JobStatus, SaveZkLinks, SaveZkNote,
    SaveZkNoteAndLinks, SavedZkNote, UuidZkLink, ZkListNote, ZkNote, ZkNoteAndLinksWhat,
    ZkNoteArchives, ZkNoteId,
  },
  search::{
    TagSearch, ZkIdSearchResult, ZkListNoteSearchResult, ZkNoteAndLinksSearchResult, ZkNoteSearch,
    ZkNoteSearchResult,
  },
};
use elm_rs::{Elm, ElmDecode, ElmEncode};
use serde_derive::{Deserialize, Serialize};
// use uuid::Uuid;

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub struct PrivateClosureRequest {
  pub closure_id: Option<i64>,
  pub request: PrivateRequest,
}
#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub struct PrivateClosureReply {
  pub closure_id: Option<i64>,
  pub reply: PrivateReply,
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub enum PrivateRequest {
  PvqGetZkNote(ZkNoteId),
  PvqGetZkNoteAndLinks(GetZkNoteAndLinks),
  PvqGetZnlIfChanged(GetZnlIfChanged),
  PvqGetZkNoteComments(GetZkNoteComments),
  PvqGetZkNoteArchives(GetZkNoteArchives),
  PvqGetArchiveZklinks(GetArchiveZkLinks),
  PvqGetZkLinksSince(GetZkLinksSince),
  PvqSearchZkNotes(ZkNoteSearch),
  PvqPowerDelete(Vec<TagSearch>),
  PvqDeleteZkNote(ZkNoteId),
  PvqSaveZkNote(SaveZkNote),
  PvqSaveZkLinks(SaveZkLinks),
  PvqSaveZkNoteAndLinks(SaveZkNoteAndLinks),
  PvqSaveImportZkNotes(Vec<ImportZkNote>),
  PvqSetHomeNote(ZkNoteId),
  PvqSyncRemote,
  PvqSyncFiles(ZkNoteSearch),
  PvqGetJobStatus(i64),
}

#[derive(Elm, ElmDecode, ElmEncode, Serialize, Deserialize, Debug)]
pub enum PrivateReply {
  PvyServerError(PrivateError),
  PvyZkNote(ZkNote),
  PvyZkNoteAndLinksWhat(ZkNoteAndLinksWhat),
  PvyNoop,
  PvyZkNoteComments(Vec<ZkNote>),
  PvyArchives(Vec<ZkListNote>),
  PvyZkNoteArchives(ZkNoteArchives),
  PvyArchiveZkLinks(Vec<ArchiveZkLink>),
  PvyZkLinks(Vec<UuidZkLink>),
  PvyZkListNoteSearchResult(ZkListNoteSearchResult),
  PvyZkNoteSearchResult(ZkNoteSearchResult),
  PvyZkNoteIdSearchResult(ZkIdSearchResult),
  PvyZkNoteAndLinksSearchResult(ZkNoteAndLinksSearchResult),
  PvyPowerDeleteComplete(i64),
  PvyDeletedZkNote(ZkNoteId),
  PvySavedZkNote(SavedZkNote),
  PvySavedZkLinks,
  PvySavedZkNoteAndLinks(SavedZkNote),
  PvySavedImportZkNotes,
  PvyHomeNoteSet(ZkNoteId),
  PvyJobStatus(JobStatus),
  PvyJobNotFound(i64),
  PvyFileSyncComplete,
  PvySyncComplete,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub struct ZkNoteRq {
  zknoteid: ZkNoteId,
  what: Option<String>,
}

#[derive(Elm, ElmDecode, ElmEncode, Deserialize, Serialize, Debug)]
pub enum PrivateError {
  PveString(String),
  PveNoteNotFound(ZkNoteRq),
  PveNoteIsPrivate(ZkNoteRq),
  PveNotLoggedIn,
  PveLoginError(String),
}
