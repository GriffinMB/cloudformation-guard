use crate::commands::validate::Reporter;
use std::io::Write;
use crate::commands::tracker::StatusContext;
use crate::rules::{Status, NamedStatus};
use colored::*;
use itertools::Itertools;
use enumflags2::{bitflags, BitFlags};
use crate::commands::validate::common::colored_string;
use crate::rules::eval_context::EventRecord;
use crate::rules::RecordType;
use std::collections::HashMap;

#[bitflags]
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
pub(super) enum SummaryType {
    PASS = 0b0001,
    FAIL = 0b0010,
    SKIP = 0b0100,
}


#[derive(Debug)]
pub(super) struct SummaryTable<'r> {
    rules_file_name: &'r str,
    data_file_name: &'r str,
    summary_type: BitFlags<SummaryType>,
}

impl<'a> SummaryTable<'a> {
    pub(crate) fn new<'r>(rules_file_name: &'r str, data_file_name: &'r str, summary_type: BitFlags<SummaryType>) -> SummaryTable<'r> {
        SummaryTable {
            rules_file_name, data_file_name, summary_type
        }
    }
}

fn print_partition(writer: &mut dyn Write,
                   rules_file_name: &str,
                   part: &[&StatusContext],
                   longest: usize) -> crate::rules::Result<()> {
    for container in part {
        writeln!(writer,
                 "{filename}/{context:<0$}{status}",
                 longest+4,
                 filename=rules_file_name,
                 context=container.context,
                 status=super::common::colored_string(container.status)
        )?;
    }
    Ok(())
}

fn print_summary(
    writer: &mut dyn Write,
    rules_file_name: &str,
    longest: usize,
    rules: &indexmap::IndexMap<&str, Status>) -> crate::rules::Result<()> {
    for (rule_name, status) in rules.iter() {
        writeln!(writer,
                 "{filename}/{context:<0$}{status}",
                 longest+4,
                 filename=rules_file_name,
                 context=rule_name,
                 status=super::common::colored_string(Some(*status)))?;
    }
    Ok(())
}


impl<'r> Reporter for SummaryTable<'r> {
    fn report(&self,
              writer: &mut dyn Write,
              status: Option<Status>,
              failed_rules: &[&StatusContext],
              passed_or_skipped: &[&StatusContext],
              longest_rule_name: usize) -> crate::rules::Result<()> {

        let as_vec = passed_or_skipped.iter().map(|s| *s)
            .collect_vec();
        let (skipped, passed): (Vec<&StatusContext>, Vec<&StatusContext>) = as_vec.iter()
            .partition(|status| match status.status { // This uses the dereference deep trait of Rust
                Some(Status::SKIP) => true,
                _ => false
            });

        writeln!(writer, "{} Status = {}", self.data_file_name, colored_string(status))?;
        if self.summary_type.contains(SummaryType::SKIP) && !skipped.is_empty() {
            writeln!(writer, "{}", "SKIP rules".bold());
            print_partition(writer, self.rules_file_name, &skipped, longest_rule_name)?;

        }

        if self.summary_type.contains(SummaryType::PASS) && !passed.is_empty() {
            writeln!(writer, "{}", "PASS rules".bold());
            print_partition(writer, self.rules_file_name, &passed, longest_rule_name)?;
        }

        if self.summary_type.contains(SummaryType::FAIL) && !failed_rules.is_empty() {
            writeln!(writer, "{}", "FAILED rules".bold());
            print_partition(writer, self.rules_file_name, failed_rules, longest_rule_name)?;
        }

        writeln!(writer, "---")?;

        Ok(())
    }

    fn report_eval(&self, writer: &mut dyn Write, status: Status, root_record: &EventRecord<'_>) -> crate::rules::Result<()> {
        writeln!(writer, "{} Status = {}", self.data_file_name, colored_string(Some(status)))?;
        let mut passed = indexmap::IndexMap::with_capacity(root_record.children.len());
        let mut skipped = indexmap::IndexMap::with_capacity(root_record.children.len());
        let mut failed = indexmap::IndexMap::with_capacity(root_record.children.len());
        let mut longest = 0;
        for each_rule in &root_record.children {
            if let Some(RecordType::RuleCheck(NamedStatus {status, name, ..})) =
                &each_rule.container {
                match status {
                    Status::PASS => passed.insert(*name, *status),
                    Status::FAIL => failed.insert(*name, *status),
                    Status::SKIP => skipped.insert(*name, *status),
                };
                if longest < name.len() {
                    longest = name.len()
                }
            }
        }

        skipped.retain(|key, _| !(passed.contains_key(key) || failed.contains_key(key)));

        if self.summary_type.contains(SummaryType::SKIP) && !skipped.is_empty() {
            writeln!(writer, "{}", "SKIP rules".bold())?;
            print_summary(writer, self.rules_file_name, longest, &skipped)?;
        }

        if self.summary_type.contains(SummaryType::PASS) && !passed.is_empty() {
            writeln!(writer, "{}", "PASS rules".bold())?;
            print_summary(writer, self.rules_file_name, longest, &passed)?;
        }

        if self.summary_type.contains(SummaryType::FAIL) && !failed.is_empty() {
            writeln!(writer, "{}", "FAILED rules".bold())?;
            print_summary(writer, self.rules_file_name, longest, &failed)?;
        }

        writeln!(writer, "---")?;
        Ok(())
    }
}