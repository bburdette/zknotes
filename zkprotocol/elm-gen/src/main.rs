use std::path::Path;

use orgauth::util;
pub use zkprotocol::content as zc;
pub use zkprotocol::private as zprv;
pub use zkprotocol::public as zpub;
pub use zkprotocol::search as zs;
pub use zkprotocol::specialnotes as sn;
pub use zkprotocol::tauri;
pub use zkprotocol::upload;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let ed = Path::new("../../elm/src");

  // --------------------------------------------------------------------------
  // Data.elm
  {
    let mut target = vec![];
    // elm_rs provides a macro for conveniently creating an Elm module with everything needed
    elm_rs::export!(
        "Data",
        &mut target,
        {        // generates types and encoders for types implementing ElmEncoder
        encoders: [zc::ZkNoteId,
                    zc::ExtraLoginData,
                    zc::ZkNote,
                    zc::FileStatus,
                    zc::ZkListNote,
                    zc::SavedZkNote,
                    zc::SaveZkNote,
                    zc::Direction,
                    zc::SaveZkLink,
                    zc::SaveZkNoteAndLinks,
                    zc::ZkLink,
                    zc::EditLink,
                    zc::ZkLinks,
                    zc::ImportZkNote,
                    zc::GetZkLinks,
                    zc::GetZkNoteAndLinks,
                    zc::GetZnlIfChanged,
                    zc::GetZkNoteArchives,
                    zc::ZkNoteArchives,
                    zc::GetArchiveZkNote,
                    zc::GetArchiveZkLinks,
                    zc::GetZkLinksSince,
                    zc::FileInfo,
                    zc::ArchiveZkLink,
                    zc::UuidZkLink,
                    zc::GetZkNoteComments,
                    zc::ZkNoteAndLinks,
                    zc::ZkNoteAndLinksWhat,
                    zc::EditTab,
                    zc::JobState,
                    zc::JobStatus,
                    zpub::PublicRequest,
                    zpub::PublicReply,
                    zpub::PublicError,
                    zprv::PrivateRequest,
                    zprv::PrivateReply,
                    zprv::PrivateError,
                    zprv::PrivateClosureRequest,
                    zprv::PrivateClosureReply,
                    zprv::ZkNoteRq,
                    upload::UploadReply,
                    zs::ZkNoteSearch,
                    zs::Ordering,
                    zs::OrderDirection,
                    zs::OrderField,
                    zs::ResultType,
                    zs::TagSearch,
                    zs::SearchMod,
                    zs::AndOr,
                    zs::ZkIdSearchResult,
                    zs::ZkListNoteSearchResult,
                    zs::ZkNoteSearchResult,
                    zs::ZkSearchResultHeader,
                    zs::ZkNoteAndLinksSearchResult,
                    zs::ArchivesOrCurrent,
                    tauri::TauriRequest,
                    tauri::TauriReply,
                    tauri::UploadedFiles,
        ]
        // generates types and decoders for types implementing ElmDecoder
        decoders: [zc::ZkNoteId,
                    zc::ExtraLoginData,
                    zc::ZkNote,
                    zc::FileStatus,
                    zc::ZkListNote,
                    zc::SavedZkNote,
                    zc::SaveZkNote,
                    zc::Direction,
                    zc::SaveZkLink,
                    zc::SaveZkNoteAndLinks,
                    zc::ZkLink,
                    zc::EditLink,
                    zc::ZkLinks,
                    zc::ImportZkNote,
                    zc::GetZkLinks,
                    zc::GetZkNoteAndLinks,
                    zc::GetZnlIfChanged,
                    zc::GetZkNoteArchives,
                    zc::ZkNoteArchives,
                    zc::GetArchiveZkNote,
                    zc::GetArchiveZkLinks,
                    zc::GetZkLinksSince,
                    zc::FileInfo,
                    zc::ArchiveZkLink,
                    zc::UuidZkLink,
                    zc::GetZkNoteComments,
                    zc::ZkNoteAndLinks,
                    zc::ZkNoteAndLinksWhat,
                    zc::EditTab,
                    zc::JobState,
                    zc::JobStatus,
                    zpub::PublicRequest,
                    zpub::PublicReply,
                    zpub::PublicError,
                    zprv::PrivateRequest,
                    zprv::PrivateReply,
                    zprv::PrivateError,
                    zprv::PrivateClosureRequest,
                    zprv::PrivateClosureReply,
                    zprv::ZkNoteRq,
                    upload::UploadReply,
                    zs::ZkNoteSearch,
                    zs::Ordering,
                    zs::OrderDirection,
                    zs::OrderField,
                    zs::ResultType,
                    zs::TagSearch,
                    zs::SearchMod,
                    zs::AndOr,
                    zs::ZkIdSearchResult,
                    zs::ZkListNoteSearchResult,
                    zs::ZkNoteSearchResult,
                    zs::ZkSearchResultHeader,
                    zs::ZkNoteAndLinksSearchResult,
                    zs::ArchivesOrCurrent,
                    tauri::TauriRequest,
                    tauri::TauriReply,
                    tauri::UploadedFiles,
        ]
        // generates types and functions for forming queries for types implementing ElmQuery
        queries: [],
        // generates types and functions for forming queries for types implementing ElmQueryField
        query_fields: [],
        }
    )
    .unwrap();

    let output = String::from_utf8(target).unwrap();

    // add line importing Orgauth.Userid
    let uidout = output.replace(
      "import Json.Encode",
      r#"import Json.Encode
import Orgauth.Data exposing (UserId(..), userIdDecoder, userIdEncoder)"#,
    );

    let outf = ed.join("Data.elm").to_str().expect("bad path").to_string();
    util::write_string(outf.as_str(), uidout.as_str())?;
    println!("wrote file: {}", outf);
  }

  // --------------------------------------------------------------------------
  // SpecialNotes.elm
  {
    let mut target = vec![];
    // elm_rs provides a macro for conveniently creating an Elm module with everything needed
    elm_rs::export!(
        "SpecialNotes",
        &mut target,
        {        // generates types and encoders for types implementing ElmEncoder
        encoders: [zc::Server,
                    sn::SpecialNote,
                    sn::Search,
                    sn::CompletedSync,
                    sn::Notelist,
        ]
        // generates types and decoders for types implementing ElmDecoder
        decoders: [zc::Server,
                    sn::SpecialNote,
                    sn::Search,
                    sn::CompletedSync,
                    sn::Notelist,
        ]
        // generates types and functions for forming queries for types implementing ElmQuery
        queries: [],
        // generates types and functions for forming queries for types implementing ElmQueryField
        query_fields: [],
        }
    )
    .unwrap();

    let output = String::from_utf8(target).unwrap();

    // add line importing Orgauth.Userid
    let uidout = output.replace(
      "import Json.Encode",
      r#"import Json.Encode
import Data exposing (TagSearch, tagSearchDecoder, tagSearchEncoder)"#,
    );

    let outf = ed
      .join("SpecialNotes.elm")
      .to_str()
      .expect("bad path")
      .to_string();
    util::write_string(outf.as_str(), uidout.as_str())?;
    println!("wrote file: {}", outf);
  }

  Ok(())
}
