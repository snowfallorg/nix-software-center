mod imp;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use std::collections::HashMap;

use crate::installed_app_row::NscInstalledAppRow;
use crate::{runtime, util};

glib::wrapper! {
    pub struct UpdatesPage(ObjectSubclass<imp::UpdatesPage>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

pub struct PackageIssue {
    pub attr: String,
    pub kind: IssueKind,
    pub message: Option<String>,
}

#[derive(strum::Display)]
pub enum IssueKind {
    #[strum(serialize = "Marked as broken")]
    Broken,
    #[strum(serialize = "Marked as insecure")]
    Insecure,
    #[strum(serialize = "No longer available")]
    Unavailable,
    #[strum(serialize = "Renamed")]
    Renamed,
    #[strum(serialize = "Removed")]
    Removed,
}

struct UpdateCheckResult {
    nixos_updates: Vec<libsnow::PackageUpdate>,
    hm_updates: Vec<libsnow::PackageUpdate>,
    profile_now_updates: Vec<libsnow::PackageUpdate>,
    profile_after_system_updates: Vec<libsnow::PackageUpdate>,
    nixos_issues: Vec<PackageIssue>,
    hm_issues: Vec<PackageIssue>,
    profile_issues: Vec<PackageIssue>,
    warnings: Vec<String>,
    current_rev: Option<String>,
    latest_rev: Option<String>,
}

impl UpdatesPage {
    pub fn check_for_updates(
        &self,
        metadata: &libsnow::metadata::Metadata,
        nixos_attrs: &[String],
        hm_attrs: &[String],
        profile_attrs: &[String],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) {
        let imp = self.imp();
        imp.loading_stack.set_visible_child_name("loading");

        let nixos_attrs = nixos_attrs.to_vec();
        let hm_attrs = hm_attrs.to_vec();
        let profile_attrs = profile_attrs.to_vec();
        let db_path = metadata.db_path().to_path_buf();
        let current_rev = metadata.nixpkgs_revision().map(|s| s.to_string());
        let current_release = metadata.nixos_release().map(|s| s.to_string());
        let (sender, receiver) = async_channel::bounded(1);

        runtime::runtime().spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(check_updates_async(
                    db_path,
                    nixos_attrs,
                    hm_attrs,
                    profile_attrs,
                    current_rev,
                    current_release,
                ))
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
            let _ = sender.send(result).await;
        });

        let page_weak = self.downgrade();
        let pkgname_map = pkgname_map.clone();
        glib::spawn_future_local(async move {
            let Ok(result) = receiver.recv().await else {
                return;
            };
            let Some(page) = page_weak.upgrade() else {
                return;
            };
            match result {
                Ok(check) => page.populate_results(&check, &pkgname_map),
                Err(err) => {
                    tracing::warn!("Update check failed: {err}");
                    let imp = page.imp();
                    imp.error_status.set_description(Some(&err));
                    imp.loading_stack.set_visible_child_name("error");
                }
            }
        });
    }

    fn populate_results(
        &self,
        result: &UpdateCheckResult,
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) {
        let imp = self.imp();

        let revs_differ = result.current_rev.is_some()
            && result.latest_rev.is_some()
            && result.current_rev != result.latest_rev;

        let nixos_count = Self::fill_appstream_update_list(
            &imp.nixos_updates_list,
            &result.nixos_updates,
            pkgname_map,
        );
        let nixos_issues = Self::append_issues_to_list(
            &imp.nixos_updates_list,
            &result.nixos_issues,
            false,
            pkgname_map,
        );
        imp.nixos_updates_section
            .set_visible(nixos_count + nixos_issues > 0);

        let hm_count =
            Self::fill_appstream_update_list(&imp.hm_updates_list, &result.hm_updates, pkgname_map);
        let hm_issues = Self::append_issues_to_list(
            &imp.hm_updates_list,
            &result.hm_issues,
            false,
            pkgname_map,
        );
        imp.hm_updates_section.set_visible(hm_count + hm_issues > 0);

        let all_system_updates: Vec<&libsnow::PackageUpdate> = result
            .nixos_updates
            .iter()
            .chain(result.hm_updates.iter())
            .collect();
        let total_system_issues = nixos_issues + hm_issues;

        let system_header_subtitle = Self::build_system_subtitle(
            &all_system_updates,
            total_system_issues,
            &result.current_rev,
            &result.latest_rev,
            pkgname_map,
        );
        imp.system_header_subtitle
            .set_label(&system_header_subtitle);
        let has_system_updates = !all_system_updates.is_empty() || total_system_issues > 0;
        imp.system_header_section
            .set_visible(has_system_updates || revs_differ);

        let profile_now_count = Self::fill_profile_app_rows(
            &imp.profile_now_list,
            &result.profile_now_updates,
            pkgname_map,
        );
        let profile_issues = Self::append_issues_to_list(
            &imp.profile_now_list,
            &result.profile_issues,
            true,
            pkgname_map,
        );
        imp.profile_now_section
            .set_visible(profile_now_count + profile_issues > 0);

        let profile_after_count = Self::fill_appstream_update_list(
            &imp.profile_after_system_list,
            &result.profile_after_system_updates,
            pkgname_map,
        );
        imp.profile_after_system_section
            .set_visible(profile_after_count > 0);

        let total_profile = profile_now_count + profile_issues + profile_after_count;
        if total_profile > 0 {
            let mut parts = Vec::new();
            if profile_now_count > 0 {
                parts.push(format!(
                    "{} {} to update now",
                    profile_now_count,
                    if profile_now_count == 1 {
                        "app"
                    } else {
                        "apps"
                    },
                ));
            }
            if profile_after_count > 0 {
                parts.push(format!("{} more after system update", profile_after_count,));
            }
            if profile_issues > 0 {
                parts.push(format!("{} with issues", profile_issues,));
            }
            imp.profile_header_subtitle.set_label(&parts.join(", "));
        }
        imp.profile_updates_section.set_visible(total_profile > 0);

        let has_system = has_system_updates || revs_differ;
        imp.profile_separator
            .set_visible(has_system && total_profile > 0);

        let has_content = has_system_updates
            || nixos_count + nixos_issues + hm_count + hm_issues + total_profile > 0
            || revs_differ;

        if has_content {
            let total_app_count = nixos_count + hm_count + profile_now_count + profile_after_count;
            let total_other_count = all_system_updates.len() - (nixos_count + hm_count);
            let total_issues = nixos_issues + hm_issues + profile_issues;

            let mut parts = Vec::new();
            if total_app_count > 0 && total_other_count > 0 {
                parts.push(format!(
                    "{} {} and {} other {} to update",
                    total_app_count,
                    if total_app_count == 1 { "app" } else { "apps" },
                    total_other_count,
                    if total_other_count == 1 {
                        "package"
                    } else {
                        "packages"
                    },
                ));
            } else if total_app_count > 0 {
                parts.push(format!(
                    "{} {} to update",
                    total_app_count,
                    if total_app_count == 1 { "app" } else { "apps" },
                ));
            } else if total_other_count > 0 {
                parts.push(format!(
                    "{} {} to update",
                    total_other_count,
                    if total_other_count == 1 {
                        "package"
                    } else {
                        "packages"
                    },
                ));
            } else if revs_differ {
                parts.push("System rebuild available".to_string());
            }
            if total_issues > 0 {
                parts.push(format!(
                    "{} {} with issues",
                    total_issues,
                    if total_issues == 1 { "app" } else { "apps" },
                ));
            }
            imp.update_everything_subtitle.set_label(&parts.join(", "));
        }
        imp.update_everything_section.set_visible(has_content);

        if !has_content && !result.warnings.is_empty() {
            imp.error_status
                .set_description(Some(&result.warnings.join("\n")));
            imp.loading_stack.set_visible_child_name("error");
        } else if !has_content {
            imp.loading_stack.set_visible_child_name("up-to-date");
        } else {
            if !result.warnings.is_empty() {
                imp.warnings_banner.set_title(&result.warnings.join("\n"));
                imp.warnings_banner.set_revealed(true);
            }
            imp.loading_stack.set_visible_child_name("content");
        }
    }

    fn fill_appstream_update_list(
        list_box: &gtk::ListBox,
        updates: &[libsnow::PackageUpdate],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> usize {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let mut count = 0;
        for update in updates {
            let attr = update.attr.to_string();
            if let Some(component) = pkgname_map.get(&attr) {
                let version_str = format!("{} → {}", update.old_version, update.new_version);
                let pkg = libsnow::Package {
                    attr: update.attr.clone(),
                    pname: None,
                    version: Some(version_str),
                    profile_name: None,
                };
                list_box.append(&NscInstalledAppRow::new(component, &pkg));
                count += 1;
            }
        }
        count
    }

    fn build_system_subtitle(
        all_updates: &[&libsnow::PackageUpdate],
        issue_count: usize,
        current_rev: &Option<String>,
        latest_rev: &Option<String>,
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> String {
        let app_count = all_updates
            .iter()
            .filter(|u| pkgname_map.contains_key(&u.attr.to_string()))
            .count();
        let other_count = all_updates.len() - app_count;

        let mut parts = Vec::new();
        if let (Some(cur), Some(lat)) = (current_rev, latest_rev)
            && cur != lat
        {
            parts.push(format!("{cur} → {lat}"));
        }
        if app_count > 0 && other_count > 0 {
            parts.push(format!(
                "{} {} and {} other {} to update",
                app_count,
                if app_count == 1 { "app" } else { "apps" },
                other_count,
                if other_count == 1 {
                    "package"
                } else {
                    "packages"
                },
            ));
        } else if app_count > 0 {
            parts.push(format!(
                "{} {} to update",
                app_count,
                if app_count == 1 { "app" } else { "apps" },
            ));
        } else if other_count > 0 {
            parts.push(format!(
                "{} {} to update",
                other_count,
                if other_count == 1 {
                    "package"
                } else {
                    "packages"
                },
            ));
        }
        if issue_count > 0 {
            parts.push(format!(
                "{} {} with issues",
                issue_count,
                if issue_count == 1 { "app" } else { "apps" },
            ));
        }

        if parts.is_empty() {
            "Up to date".to_string()
        } else {
            parts.join("\n")
        }
    }

    fn fill_profile_app_rows(
        list_box: &gtk::ListBox,
        updates: &[libsnow::PackageUpdate],
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> usize {
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let mut count = 0;
        for update in updates {
            let attr = update.attr.to_string();
            if let Some(component) = pkgname_map.get(&attr) {
                let version_str = format!("{} → {}", update.old_version, update.new_version);
                let pkg = libsnow::Package {
                    attr: update.attr.clone(),
                    pname: None,
                    version: Some(version_str),
                    profile_name: None,
                };
                let row = NscInstalledAppRow::new(component, &pkg);
                row.add_action(
                    &gtk::Button::builder()
                        .icon_name("software-update-available-symbolic")
                        .css_classes(["flat"])
                        .valign(gtk::Align::Center)
                        .tooltip_text("Update")
                        .build(),
                );
                list_box.append(&row);
                count += 1;
            }
        }
        count
    }

    fn append_issues_to_list(
        list_box: &gtk::ListBox,
        issues: &[PackageIssue],
        is_profile: bool,
        pkgname_map: &HashMap<String, libappstream::Component>,
    ) -> usize {
        let mut count = 0;
        for issue in issues {
            let Some(component) = pkgname_map.get(&issue.attr) else {
                continue;
            };
            let fallback = issue.kind.to_string();
            let label = issue.message.as_deref().unwrap_or(&fallback);
            let pkg = libsnow::Package {
                attr: libsnow::PackageAttr::NixPkgs {
                    attr: issue.attr.clone(),
                },
                pname: None,
                version: Some(label.to_string()),
                profile_name: None,
            };
            let row = NscInstalledAppRow::new(component, &pkg);
            row.add_css_class("issue-row");
            if is_profile {
                row.add_action(
                    &gtk::Button::builder()
                        .icon_name("user-trash-symbolic")
                        .css_classes(["flat"])
                        .valign(gtk::Align::Center)
                        .tooltip_text("Remove")
                        .build(),
                );
            }
            list_box.append(&row);
            count += 1;
        }
        count
    }
}

async fn check_updates_async(
    db_path: std::path::PathBuf,
    nixos_attrs: Vec<String>,
    hm_attrs: Vec<String>,
    profile_attrs: Vec<String>,
    current_rev: Option<String>,
    current_release: Option<String>,
) -> Result<UpdateCheckResult, String> {
    let md = libsnow::metadata::Metadata::open(&db_path).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();

    let nixos_updates = match libsnow::nixos::update::updatable(&md).await {
        Ok(u) => u,
        Err(err) => {
            tracing::warn!("Failed to check NixOS updates: {err}");
            warnings.push(format!("Could not check NixOS updates: {err}"));
            Vec::new()
        }
    };
    let hm_updates = match libsnow::homemanager::update::updatable(&md).await {
        Ok(u) => u,
        Err(err) => {
            tracing::warn!("Failed to check Home Manager updates: {err}");
            warnings.push(format!("Could not check Home Manager updates: {err}"));
            Vec::new()
        }
    };
    let profile_now_updates = match libsnow::profile::update::updatable_user().await {
        Ok(u) => u,
        Err(err) => {
            tracing::warn!("Failed to check profile updates (registry): {err}");
            warnings.push(format!("Could not check profile updates: {err}"));
            Vec::new()
        }
    };
    let profile_latest_updates = match libsnow::profile::update::updatable().await {
        Ok(u) => u,
        Err(err) => {
            tracing::warn!("Failed to check profile updates (latest): {err}");
            Vec::new()
        }
    };

    let now_attrs: std::collections::HashMap<String, &str> = profile_now_updates
        .iter()
        .map(|u| (u.attr.to_string(), u.new_version.as_str()))
        .collect();
    let profile_after_system_updates: Vec<libsnow::PackageUpdate> = profile_latest_updates
        .into_iter()
        .filter(|u| {
            let attr = u.attr.to_string();
            match now_attrs.get(&attr) {
                Some(now_ver) => *now_ver != u.new_version,
                None => true,
            }
        })
        .collect();

    let (nixos_issues, hm_issues, latest_rev, latest_release) =
        match libsnow::metadata::Metadata::connect_latest().await {
            Ok(latest_md) => {
                let ni = detect_issues(&latest_md, &nixos_attrs);
                let hi = detect_issues(&latest_md, &hm_attrs);
                let rev = latest_md.nixpkgs_revision().map(|s| s.to_string());
                let release = latest_md.nixos_release().map(|s| s.to_string());
                (ni, hi, rev, release)
            }
            Err(err) => {
                tracing::warn!("Could not fetch latest metadata for issue detection: {err}");
                warnings.push(format!("Could not check for package issues: {err}"));
                (Vec::new(), Vec::new(), None, None)
            }
        };

    let profile_issues = match libsnow::metadata::Metadata::connect_registry().await {
        Ok(registry_md) => detect_issues(&registry_md, &profile_attrs),
        Err(err) => {
            tracing::warn!("Could not fetch registry metadata for profile issue detection: {err}");
            warnings.push(format!("Could not check for profile package issues: {err}"));
            Vec::new()
        }
    };

    let format_rev = |rev: &Option<String>, release: &Option<String>| -> Option<String> {
        match (release, rev) {
            (Some(rel), Some(r)) => Some(format!("{} ({})", rel, &r[..7.min(r.len())])),
            (None, Some(r)) => Some(r[..7.min(r.len())].to_string()),
            _ => None,
        }
    };

    Ok(UpdateCheckResult {
        nixos_updates,
        hm_updates,
        profile_now_updates,
        profile_after_system_updates,
        nixos_issues,
        hm_issues,
        profile_issues,
        warnings,
        current_rev: format_rev(&current_rev, &current_release),
        latest_rev: format_rev(&latest_rev, &latest_release),
    })
}

fn detect_issues(md: &libsnow::metadata::Metadata, attrs: &[String]) -> Vec<PackageIssue> {
    let mut issues = Vec::new();

    for attr in attrs {
        let stripped = util::strip_nix_output_suffix(attr);
        let pkg_result = md.get(attr).or_else(|e| {
            if stripped != attr {
                md.get(stripped)
            } else {
                Err(e)
            }
        });

        match pkg_result {
            Ok(pkg_info) => {
                if pkg_info.broken {
                    issues.push(PackageIssue {
                        attr: attr.clone(),
                        kind: IssueKind::Broken,
                        message: Some("Package is marked as broken".to_string()),
                    });
                }
                if pkg_info.insecure {
                    issues.push(PackageIssue {
                        attr: attr.clone(),
                        kind: IssueKind::Insecure,
                        message: Some("Package is marked as insecure".to_string()),
                    });
                }
            }
            Err(_) => {
                let alias = md.get_alias(attr).or_else(|| {
                    let stripped = util::strip_nix_output_suffix(attr);
                    if stripped != attr {
                        md.get_alias(stripped)
                    } else {
                        None
                    }
                });
                if let Some(alias) = alias {
                    match alias.kind {
                        libsnow::metadata::AliasKind::Rename {
                            replacement,
                            message,
                        } => {
                            issues.push(PackageIssue {
                                attr: attr.clone(),
                                kind: IssueKind::Renamed,
                                message: Some(
                                    message.unwrap_or_else(|| format!("Renamed to {replacement}")),
                                ),
                            });
                        }
                        libsnow::metadata::AliasKind::Removed { message } => {
                            issues.push(PackageIssue {
                                attr: attr.clone(),
                                kind: IssueKind::Removed,
                                message: Some(message),
                            });
                        }
                    }
                } else {
                    issues.push(PackageIssue {
                        attr: attr.clone(),
                        kind: IssueKind::Unavailable,
                        message: None,
                    });
                }
            }
        }
    }

    issues
}
