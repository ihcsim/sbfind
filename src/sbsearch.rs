use chrono::{self, DateTime, Utc};
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, SearcherBuilder, sinks::UTF8};
use log::*;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::fs::{self};
use std::io::{self, Read};
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct Entry {
    pub level: String,
    pub path: String,
    pub content: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub log_type: LogType,
}

impl Entry {
    fn from_str(s: &str, path: &str, sbsearch: &SBSearch) -> Entry {
        let mut timestamp: Option<DateTime<Utc>> = None;
        if let Ok(t) = sbsearch.find_timestamp(s) {
            timestamp = t;
        }

        let mut level = "UNKNOWN";
        if let Ok(r) = sbsearch.find_log_level(s) {
            level = r;
        }

        let log_type = sbsearch.log_type(path);
        Entry {
            content: String::from(s),
            level: String::from(level),
            path: String::from(path),
            log_type,
            timestamp,
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = self.content.clone();
        write!(f, "{}", out)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum LogType {
    #[default]
    Workload,

    System,
}

impl fmt::Display for LogType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self == &LogType::Workload {
            write!(f, "Workload")
        } else {
            write!(f, "System")
        }
    }
}

#[derive(Debug)]
pub struct SearchCache {
    pub all: Vec<Entry>,
    pub system_entries: Vec<Entry>,
    pub workload_entries: Vec<Entry>,
}

#[derive(Debug)]
pub struct SearchResult {
    pub system_entries_offset: Vec<Entry>,
    pub workload_entries_offset: Vec<Entry>,
}

pub fn search(dir: &Path, keyword: &str, cache: &mut SearchCache) -> Result<(), Box<dyn Error>> {
    let root_dir = dir.to_str().unwrap();
    let mut sbsearch = SBSearch::new(root_dir, keyword)?;
    sbsearch.search_tree(dir, &mut cache.all)?;
    cache.all.sort_by(|a, b| {
        // entries with incomplete or no timestamp are placed at the end
        if a.timestamp.is_none() && b.timestamp.is_some() {
            std::cmp::Ordering::Greater
        } else if b.timestamp.is_none() && a.timestamp.is_some() {
            std::cmp::Ordering::Less
        } else {
            a.timestamp.cmp(&b.timestamp)
        }
    });

    // split system entries and workload entries into two vectors for different views
    cache.all.iter().for_each(|entry| {
        if entry.log_type == LogType::System {
            cache.system_entries.push(entry.clone());
        } else if entry.log_type == LogType::Workload {
            cache.workload_entries.push(entry.clone());
        }
    });
    Ok(())
}

fn is_zip(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut signature = [0u8; 4];
    match file.read_exact(&mut signature) {
        Ok(_) => Ok(signature == [0x50, 0x4B, 0x03, 0x04]),
        Err(_) => Ok(false),
    }
}

struct SBSearch {
    searcher: Searcher,
    root_dir: String,
    matcher_keyword: RegexMatcher,
    matcher_log_level1: RegexMatcher,
    matcher_log_level2: RegexMatcher,
    matcher_log_level3: RegexMatcher,
    matcher_log_level4: RegexMatcher,
    matcher_log_level5: RegexMatcher,
    matcher_timestamp1: RegexMatcher,
    matcher_timestamp2: RegexMatcher,
}

impl SBSearch {
    fn new(root_dir: &str, keyword: &str) -> Result<Self, Box<dyn Error>> {
        let searcher: Searcher;
        unsafe {
            let mmap_choice = grep_searcher::MmapChoice::auto();
            searcher = SearcherBuilder::new()
                .memory_map(mmap_choice)
                .heap_limit(Some(268435456))
                .build();
        }
        let pattern = String::from(".*") + keyword + ".*";
        let matcher_keyword = RegexMatcher::new(pattern.as_str())?;
        let matcher_log_level1 = RegexMatcher::new(r"level=([^\s]+)")?;
        let matcher_log_level2 = RegexMatcher::new(r#""level":"([^"]+)""#)?;
        let matcher_log_level3 = RegexMatcher::new(r"err=")?;
        let matcher_log_level4 = RegexMatcher::new(r"(?i)\[error\]")?;
        let matcher_log_level5 = RegexMatcher::new(r"rpc error")?;
        let matcher_timestamp1 =
            RegexMatcher::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z")?;
        let matcher_timestamp2 = RegexMatcher::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d{3}")?;
        Ok(SBSearch {
            searcher,
            root_dir: String::from(root_dir),
            matcher_keyword,
            matcher_log_level1,
            matcher_log_level2,
            matcher_log_level3,
            matcher_log_level4,
            matcher_log_level5,
            matcher_timestamp1,
            matcher_timestamp2,
        })
    }

    fn search_tree(&mut self, dir: &Path, entries: &mut Vec<Entry>) -> Result<(), Box<dyn Error>> {
        // only search '/logs' and '/nodes/*/logs' directories
        if !self.is_log_dir(dir) {
            debug!("skipping directory: {}", dir.display());
            return Ok(());
        }
        info!("search directory: {}", dir.display());

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                debug!("entering directory: {}", path.display());
                self.search_tree(&path, entries)?;
                continue;
            }

            if path.is_file() {
                let searcher = &mut self.searcher.clone();
                if is_zip(path.as_path())? {
                    debug!("examining zip archive: {}", path.display());
                    let zipfile = File::open(&path)?;
                    let mut archive = ZipArchive::new(zipfile)?;

                    // examine each file in the zip archive in memory
                    for index in 0..archive.len() {
                        let reader = archive.by_index(index)?;
                        let path = path.join(Path::new(reader.name()));

                        debug!("examining archive file: {}", path.display());
                        self.search_reader(reader, path.as_path(), entries, searcher)?;
                    }
                    continue;
                }

                debug!("examining file: {}", path.display());
                self.search_file(&path, entries, searcher)?;
                continue;
            }
        }
        Ok(())
    }

    fn search_file(
        &self,
        path: &Path,
        entries: &mut Vec<Entry>,
        searcher: &mut Searcher,
    ) -> Result<(), Box<dyn Error>> {
        searcher.search_path(
            &self.matcher_keyword,
            path,
            UTF8(|_lnum, line| {
                let path = path.to_str().unwrap_or("");
                debug!("found matching entry in file {}", path);

                let entry = Entry::from_str(line, path, self);
                debug!("entry: {:?}", entry);

                entries.push(entry);
                Ok(true)
            }),
        )?;
        Ok(())
    }

    fn search_reader<R>(
        &mut self,
        read_from: R,
        path: &Path,
        entries: &mut Vec<Entry>,
        searcher: &mut Searcher,
    ) -> Result<(), Box<dyn Error>>
    where
        R: Read,
    {
        searcher.search_reader(
            &self.matcher_keyword,
            read_from,
            UTF8(|_lnum, line| {
                let path = path.to_str().unwrap_or("");
                debug!("found matching entry in file {}", path);

                let entry = Entry::from_str(line, path, self);
                debug!("entry: {:?}", entry);

                entries.push(entry);
                Ok(true)
            }),
        )?;
        Ok(())
    }

    fn is_log_dir(&self, dir: &Path) -> bool {
        let root_dir = Path::new(self.root_dir.as_str());
        if dir == root_dir || dir == root_dir.join("logs") || dir == root_dir.join("nodes") {
            return true;
        } else {
            for ancestor in dir.ancestors() {
                if let Some(path) = ancestor.to_str()
                    && path.contains("/logs")
                {
                    return true;
                }
            }
        }
        false
    }

    fn find_log_level<'a>(&self, line: &'a str) -> Result<&'a str, Box<dyn Error>> {
        if let Ok(opt) = self.matcher_log_level1.find(line.as_bytes())
            && let Some(m) = opt
        {
            Ok(line[m.start()..m.end()].split('=').nth(1).unwrap())
        } else if let Ok(opt) = self.matcher_log_level2.find(line.as_bytes())
            && let Some(m) = opt
        {
            Ok(line[m.start()..m.end()]
                .split(':')
                .nth(1)
                .unwrap()
                .trim_matches('"'))
        } else if let Ok(opt) = self.matcher_log_level3.find(line.as_bytes())
            && opt.is_some()
        {
            Ok("error")
        } else if let Ok(opt) = self.matcher_log_level4.find(line.as_bytes())
            && opt.is_some()
        {
            Ok("error")
        } else if let Ok(opt) = self.matcher_log_level5.find(line.as_bytes())
            && opt.is_some()
        {
            Ok("error")
        } else {
            Ok("UNKNOWN")
        }
    }

    fn find_timestamp(&self, line: &str) -> Result<Option<DateTime<Utc>>, Box<dyn Error>> {
        if let Some(m) = self.matcher_timestamp1.find(line.as_bytes())? {
            Ok(Some(DateTime::parse_from_rfc3339(&line[m])?.to_utc()))
        } else if let Some(m) = self.matcher_timestamp2.find(line.as_bytes())? {
            let naive = chrono::NaiveDateTime::parse_from_str(&line[m], "%Y-%m-%d %H:%M:%S%.f")?;
            Ok(Some(naive.and_utc()))
        } else {
            Ok(None)
        }
    }

    fn log_type(&self, path: &str) -> LogType {
        if path.contains("/nodes/") {
            LogType::System
        } else {
            LogType::Workload
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_workload_entries() {
        let path = Path::new("testdata/support_bundle");
        let keyword = "vm-00";
        let mut cache = SearchCache {
            all: Vec::new(),
            system_entries: Vec::new(),
            workload_entries: Vec::new(),
        };
        assert!(search(path, keyword, &mut cache).is_ok());
        assert_eq!(cache.all.len(), 244);

        let workload_entries = &cache.workload_entries;
        assert!(!workload_entries.is_empty());
        assert_eq!(workload_entries.len(), 218);

        // validate the first workload entry in the search result
        assert_eq!(workload_entries[0].level, "info");
        assert_eq!(
            workload_entries[0].path,
            "testdata/support_bundle/logs/harvester-system/harvester-webhook-6cb965f6d9-z24qs/harvester-webhook.log",
        );
        assert_eq!(
            workload_entries[0].content.trim_end(),
            r#"2025-12-30T21:57:51.388772685Z time="2025-12-30T21:57:51Z" level=info msg="PVC default/vm-00-disk-0-xx3er is not related to the VM image, skip patch""#
        );
        assert_eq!(
            workload_entries[0].timestamp.unwrap(),
            "2025-12-30T21:57:51.388772685Z"
                .parse::<DateTime<Utc>>()
                .unwrap()
        );

        // validate the last workload entry in the search result
        let last_index = workload_entries.len() - 1;
        assert_eq!(workload_entries[last_index].level, "info");
        assert_eq!(
            workload_entries[last_index].path,
            "testdata/support_bundle/logs/default/virt-launcher-vm-00-pb825/compute.log",
        );
        assert_eq!(
            workload_entries[last_index].content.trim_end(),
            r#"2025-12-30T22:00:42.449112443Z {"component":"virt-launcher","kind":"","level":"info","msg":"Synced vmi","name":"vm-00","namespace":"default","pos":"server.go:208","timestamp":"2025-12-30T22:00:42.448989Z","uid":"86079a85-5289-4e46-88ce-871a9eb2c0ae"}"#
        );
        assert_eq!(
            workload_entries[last_index].timestamp.unwrap(),
            "2025-12-30T22:00:42.449112443Z"
                .parse::<DateTime<Utc>>()
                .unwrap()
        );
    }

    #[test]
    fn test_search_system_entries() {
        let path = Path::new("testdata/support_bundle");
        let keyword = "vm-00";
        let mut cache = SearchCache {
            all: Vec::new(),
            system_entries: Vec::new(),
            workload_entries: Vec::new(),
        };
        assert!(search(path, keyword, &mut cache).is_ok());
        assert_eq!(cache.all.len(), 244);

        let system_entries = &cache.system_entries;
        assert!(!system_entries.is_empty());
        assert_eq!(system_entries.len(), 26);

        // validate the first workload entry in the search result
        assert_eq!(system_entries[0].level, "info");
        assert_eq!(
            system_entries[0].path,
            "testdata/support_bundle/nodes/isim-dev.zip/isim-dev/logs/containerd.log",
        );
        assert_eq!(
            system_entries[0].content.trim_end(),
            r#"time="2025-12-30T21:58:14.213504533Z" level=info msg="RunPodSandbox for name:\"virt-launcher-vm-00-pb825\"  uid:\"e0762618-5577-4082-9f9e-eaa13b7521fa\"  namespace:\"default\"""#
        );
        assert_eq!(
            system_entries[0].timestamp.unwrap(),
            "2025-12-30T21:58:14.213504533Z"
                .parse::<DateTime<Utc>>()
                .unwrap()
        );

        // validate the last workload entry in the search result
        let last_index = system_entries.len() - 1;
        assert_eq!(system_entries[last_index].level, "UNKNOWN");
        assert_eq!(
            system_entries[last_index].path,
            "testdata/support_bundle/nodes/isim-dev.zip/isim-dev/logs/containerd.log",
        );
        assert_eq!(
            system_entries[last_index].content.trim_end(),
            r#"I1230 21:58:14.297331   52196 event.go:377] Event(v1.ObjectReference{Kind:"Pod", Namespace:"default", Name:"virt-launcher-vm-00-pb825", UID:"e0762618-5577-4082-9f9e-eaa13b7521fa", APIVersion:"v1", ResourceVersion:"12670", FieldPath:""}): type: 'Normal' reason: 'AddedInterface' Add eth0 [10.52.0.87/32] from k8s-pod-network"#
        );
        assert!(system_entries[last_index].timestamp.is_none());
    }

    #[test]
    fn test_find_log_level_pattern1() {
        let sb_search = SBSearch::new("./testdata/support_bundle", "test").unwrap();

        let line = r#"2025-12-08T07:35:14.665171218Z ts=2025-12-08T07:35:14.665Z caller=kubernetes.go:331 level=info component="discovery manager scrape" discovery=kubernetes config=serviceMonitor/cattle-fleet-system/monitoring-fleet-controller/0 msg="Using pod service account via in-cluster config"#;
        let expected = "info";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T07:35:16.192939534Z time="2025-12-08T07:35:16Z" level=info msg="Diff: [docker.io/rancher/harvester-node-disk-manager-webhook:v0.7.11 docker.io/rancher/harvester:v1.4.3 docker.io/rancher/kubectl:v1.21.5 ghcr.io/k8snetworkplumbingwg/whereabouts:v0.7.0 docker.io/longhornio/csi-node-driver-registrar:v2.13.0 docker.io/longhornio/longhorn-cli:v1.7.3 docker.io/rancher/hardened-flannel:v0.26.5-build20250306 docker.io/rancher/harvester-network-controller:v0.5.6 docker.io/rancher/mirrored-jimmidyson-configmap-reload:v0.4.0 docker.io/rancher/system-agent-installer-rancher:v2.10.1 docker.io/rancher/system-agent:v0.3.11-suc docker.io/longhornio/support-bundle-kit:v0.0.51 docker.io/rancher/harvester-node-manager:v0.3.4 docker.io/rancher/mirrored-grafana-grafana:9.1.5 docker.io/rancher/fleet:v0.11.2 docker.io/rancher/harvester-load-balancer-webhook:v0.4.4 docker.io/rancher/mirrored-kiwigrid-k8s-sidecar:1.24.6 docker.io/longhornio/csi-attacher:v4.8.0 docker.io/rancher/harvester-network-helper:v0.5.6 docker.io/rancher/mirrored-prometheus-operator-prometheus-operator:v0.65.1 docker.io/rancher/shell:v0.1.26 docker.io/rancher/mirrored-kube-state-metrics-kube-state-metrics:v2.10.1 docker.io/rancher/nginx-ingress-controller:v1.12.1-hardened1 docker.io/rancher/rancher-agent:v2.10.1 docker.io/longhornio/backing-image-manager:v1.7.3 docker.io/longhornio/longhorn-manager:v1.7.3 docker.io/longhornio/longhorn-ui:v1.7.3 docker.io/rancher/fleet-agent:v0.11.2 docker.io/rancher/system-upgrade-controller:v0.14.2 ghcr.io/kube-logging/config-reloader:v0.0.5 registry.suse.com/suse/sles/15.6/virt-controller:1.3.1-150600.5.9.1 docker.io/rancher/harvester-networkfs-manager:v0.1.2 docker.io/rancher/harvester-pcidevices:v0.4.3 docker.io/rancher/harvester-webhook:v1.4.3 docker.io/rancher/rancher-webhook:v0.6.2 docker.io/longhornio/csi-snapshotter:v7.0.2-20250204 docker.io/rancher/hardened-dns-node-cache:1.24.0-build20241211 docker.io/rancher/harvester-eventrouter:v0.3.3 registry.suse.com/suse/sles/15.6/virt-launcher:1.3.1-150600.5.9.1 docker.io/rancher/harvester-node-manager-webhook:v0.3.4 docker.io/rancher/mirrored-kube-logging-logging-operator:4.4.0 docker.io/rancher/mirrored-prometheus-adapter-prometheus-adapter:v0.10.0 docker.io/rancher/kubectl:v1.20.2 docker.io/rancher/harvester-node-disk-manager:v0.7.11 docker.io/rancher/mirrored-ingress-nginx-kube-webhook-certgen:v20221220-controller-v1.5.1-58-g787ea74b6 docker.io/rancher/mirrored-prometheus-operator-prometheus-config-reloader:v0.65.1 docker.io/rancher/hardened-etcd:v3.5.19-k3s1-build20250306 docker.io/rancher/hardened-kubernetes:v1.31.7-rke2r1-build20250312 docker.io/rancher/hardened-multus-cni:v4.1.4-build20250108 registry.suse.com/suse/sles/15.6/libguestfs-tools:1.3.1-150600.5.9.1 registry.suse.com/suse/sles/15.6/virt-operator:1.3.1-150600.5.9.1 docker.io/rancher/hardened-cluster-autoscaler:v1.9.0-build20241126 docker.io/rancher/harvester-cluster-repo:v1.4.3 docker.io/rancher/harvester-network-webhook:v0.5.6 docker.io/rancher/harvester-vm-import-controller:v0.4.3 docker.io/rancher/shell:v0.1.24 registry.suse.com/suse/sles/15.6/virt-api:1.3.1-150600.5.9.1 docker.io/fluent/fluent-bit:2.1.8 docker.io/longhornio/csi-provisioner:v4.0.1-20250204 docker.io/rancher/harvester-load-balancer:v0.4.4 docker.io/rancher/mirrored-prometheus-node-exporter:v1.3.1 docker.io/longhornio/csi-resizer:v1.13.1 docker.io/rancher/rke2-cloud-provider:v1.31.2-0.20241016053446-0955fa330f90-build20241016 docker.io/longhornio/livenessprobe:v2.15.0 docker.io/rancher/rke2-runtime:v1.31.7-rke2r1 registry.suse.com/suse/sles/15.6/virt-handler:1.3.1-150600.5.9.1 docker.io/rancher/hardened-calico:v3.29.2-build20250306 docker.io/rancher/mirrored-cluster-api-controller:v1.8.3 docker.io/rancher/mirrored-prometheus-prometheus:v2.45.0 docker.io/rancher/rancher:v2.10.1 docker.io/rancher/harvester-seeder:v0.4.3 docker.io/rancher/mirrored-prometheus-alertmanager:v0.26.0 docker.io/rancher/system-agent-installer-rke2:v1.31.7-rke2r1 ghcr.io/kube-logging/fluentd:v1.15-ruby3 docker.io/rancher/klipper-helm:v0.9.4-build20250113 docker.io/longhornio/longhorn-share-manager:v1.7.3 docker.io/rancher/hardened-coredns:v1.12.0-build20241126]"#;
        let expected = "info";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T07:55:50.064883108Z time="2025-12-08T07:55:50Z" level=error msg="error syncing 'fleet-local/request-x49zj': handler cluster-registration: failed to delete fleet-local/request-x49zj rbac.authorization.k8s.io/v1, Kind=RoleBinding for cluster-registration fleet-local/request-x49zj: rolebindings.rbac.authorization.k8s.io \"request-x49zj\" not found, requeuing"#;
        let expected = "error";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T10:30:36.714032412Z time="2025-12-08T10:30:36Z" level=debug msg="Prepare to encode to yaml file path: /tmp/support-bundle-kit/bundle/yamls/namespaced/fleet-local/v1/configmaps.yaml"#;
        let expected = "debug";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_log_level_pattern2() {
        let sb_search = SBSearch::new("./testdata/support_bundle", "test").unwrap();

        let line = r#"2025-12-08T07:31:53.675701835Z {"level":"warn","ts":"2025-12-08T07:31:53.675659Z","caller":"etcdserver/util.go:170","msg":"apply request took too long","took":"122.37201ms","expected-duration":"100ms","prefix":"read-only range ","request":"key:\"/registry/pods/cattle-fleet-local-system/fleet-agent-77c65c9d9d-pxttp\" limit:1 ","response":"range_response_count:0 size:7"}"#;
        let expected = "warn";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T07:31:53.675709316Z {"level":"info","ts":"2025-12-08T07:31:53.675686Z","caller":"traceutil/trace.go:171","msg":"trace[1928396386] range","detail":"{range_begin:/registry/pods/cattle-fleet-local-system/fleet-agent-77c65c9d9d-pxttp; range_end:; response_count:0; response_revision:89089900; }","duration":"122.440061ms","start":"2025-12-08T07:31:53.553239Z","end":"2025-12-08T07:31:53.675679Z","steps":["trace[1928396386] 'agreement among raft nodes before linearized reading'  (duration: 122.37561ms)"],"step_count":1}"#;
        let expected = "info";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T10:27:24.459805082Z {"level":"info","ts":"2025-12-08T10:27:24Z","logger":"bundle","msg":"Unchanged bundledeployment","controller":"bundle","controllerGroup":"fleet.cattle.io","controllerKind":"Bundle","Bundle":{"name":"mcc-rancher-monitoring-crd","namespace":"fleet-local"},"namespace":"fleet-local","name":"mcc-rancher-monitoring-crd","reconcileID":"60a1cd4d-9ddf-4248-a6c6-c1353dab3e71","manifestID":"s-f2fb94554dbed0b86084cd509f78763ed14e1338a52bd90ee7a4b7ff53e0a","bundledeployment":{"metadata":{"name":"mcc-rancher-monitoring-crd","namespace":"cluster-fleet-local-local-1a3d67d0a899","creationTimestamp":null,"labels":{"fleet.cattle.io/bundle-name":"mcc-rancher-monitoring-crd","fleet.cattle.io/bundle-namespace":"fleet-local","fleet.cattle.io/cluster":"local","fleet.cattle.io/cluster-namespace":"fleet-local","fleet.cattle.io/managed":"true"},"finalizers":["fleet.cattle.io/bundle-deployment-finalizer"]},"spec":{"paused":true,"stagedOptions":{"defaultNamespace":"cattle-monitoring-system","helm":{"releaseName":"rancher-monitoring-crd","version":"105.1.2+up61.3.2","timeoutSeconds":600},"ignore":{}},"stagedDeploymentID":"s-f2fb94554dbed0b86084cd509f78763ed14e1338a52bd90ee7a4b7ff53e0a:90a578a64e92227563052c8bf1f175c182d754a1955e3222f1b8f6dcdabb5ee8","options":{"defaultNamespace":"cattle-monitoring-system","helm":{"releaseName":"rancher-monitoring-crd","version":"105.1.2+up61.3.2","timeoutSeconds":600},"ignore":{}},"deploymentID":"s-f2fb94554dbed0b86084cd509f78763ed14e1338a52bd90ee7a4b7ff53e0a:90a578a64e92227563052c8bf1f175c182d754a1955e3222f1b8f6dcdabb5ee8"},"status":{"display":{},"resourceCounts":{"ready":0,"desiredReady":0,"waitApplied":0,"modified":0,"orphaned":0,"missing":0,"unknown":0,"notReady":0}}},"deploymentID":"s-f2fb94554dbed0b86084cd509f78763ed14e1338a52bd90ee7a4b7ff53e0a:90a578a64e92227563052c8bf1f175c182d754a1955e3222f1b8f6dcdabb5ee8","operation":"unchanged"}"#;
        let expected = "info";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_log_level_pattern3() {
        let sb_search = SBSearch::new("./testdata/support_bundle", "test").unwrap();
        let line = r#"2025-12-08T07:27:14.834602400Z E1208 07:27:14.834539       1 job_controller.go:631] "Unhandled Error" err="syncing job: tracking status: adding uncounted pods to status: Operation cannot be fulfilled on jobs.batch \"fleet-cleanup-clusterregistrations\": the object has been modified; please apply your changes to the latest version and try again" logger="UnhandledError"
"#;
        let expected = "error";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_find_log_level_pattern4() {
        let sb_search = SBSearch::new("./testdata/support_bundle", "test").unwrap();
        let line = r#"2025-12-08T07:47:45.565219601Z 2025/12/08 07:47:45 [error] 3099#3099: *7756 upstream prematurely closed connection while reading upstream, client: 192.168.48.101, server: rancher.192.168.48.100.example.org, request: "GET /apis/fleet.cattle.io/v1alpha1/namespaces/cluster-fleet-default-mgmt-bb69eaf374c2/bundledeployments?allowWatchBookmarks=true&resourceVersion=20055629&timeoutSeconds=479&watch=true HTTP/2.0", upstream: "http://10.52.0.2:80/apis/fleet.cattle.io/v1alpha1/namespaces/cluster-fleet-default-mgmt-bb69eaf374c2/bundledeployments?allowWatchBookmarks=true&resourceVersion=20055629&timeoutSeconds=479&watch=true", host: "rancher.192.168.48.100.example.org"
"#;
        let expected = "error";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T08:23:35.438311029Z 2025/12/08 08:23:35 [ERROR] error syncing 'fleet-local/local-managed-system-upgrade-controller': handler mcc-bundle: configmaps "" not found, requeuing"#;
        let expected = "error";
        let actual = sb_search.find_log_level(line).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_included_path() {
        let sb_search = SBSearch::new("testdata/support_bundle", "").unwrap();
        let path = Path::new("testdata/support_bundle");
        assert!(sb_search.is_log_dir(path));

        let path =
            Path::new("testdata/support_bundle/logs/kube-system/rke2-canal-jnjvb/calico-node.log");
        assert!(sb_search.is_log_dir(path));

        let path = Path::new(
            "testdata/support_bundle/logs/harvester-system/harvester-webhook-6cb965f6d9-z24qs/harvester-webhook.log",
        );
        assert!(sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/nodes");
        assert!(sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/nodes/node1/logs/kubelet.log");
        assert!(sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/nodes/node1.zip");
        assert!(!sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/nodes/node1/kubelet.log");
        assert!(!sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/nodes/node2/somefile.txt");
        assert!(!sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/yamls");
        assert!(!sb_search.is_log_dir(path));

        let path = Path::new("testdata/support_bundle/yamls/namespaced/default/pods.yaml");
        assert!(!sb_search.is_log_dir(path));
    }

    #[test]
    fn test_find_timestamp() {
        let sb_search = SBSearch::new("./testdata/support_bundle", "").unwrap();
        let line = r#"2025-12-08T08:23:35.438311029Z 2025/12/08 08:23:35 [ERROR] error syncing 'fleet-local/local-managed-system-upgrade-controller': handler mcc-bundle: configmaps "" not found, requeuing"#;
        let expected = "2025-12-08T08:23:35.438311029Z"
            .parse::<DateTime<Utc>>()
            .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"2025-12-08T07:47:45.565219601Z 2025/12/08 07:47:45 [error] 3099#3099: *7756 upstream prematurely closed connection while reading upstream, client: 192.168.48.101, server: rancher.192.168.48.100.example.org, request: "GET /apis/fleet.cattle.io/v1alpha1/namespaces/cluster-fleet-default-mgmt-bb69eaf374c2/bundledeployments?allowWatchBookmarks=true&resourceVersion=20055629&timeoutSeconds=479&watch=true HTTP/2.0", upstream: "http://10.52.0.2:80/apis/fleet.cattle.io/v1alpha1/namespaces/cluster-fleet-default-mgmt-bb69eaf374c2/bundledeployments?allowWatchBookmarks=true&resourceVersion=20055629&timeoutSeconds=479&watch=true", host: "rancher.192.168.48.100.example.org"#;
        let expected = "2025-12-08T07:47:45.565219601Z"
            .parse::<DateTime<Utc>>()
            .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"testdata/support_bundle_backup/nodes/isim-dev/logs/containerd.log:3872:2025-12-30 21:58:14.266 [INFO][52211] cni-plugin/dataplane_linux.go 508: Disabling IPv4 forwarding ContainerID="41c85156546ac63f9402d1356a4d2dc00c4b807eed439c51678d1b94fac16f7c" Namespace="default" Pod="virt-launcher-vm-00-pb825" WorkloadEndpoint="isim--dev-k8s-virt--launcher--vm--00--pb825-eth0""#;
        let expected = chrono::NaiveDateTime::parse_from_str(
            "2025-12-30 21:58:14.266",
            "%Y-%m-%d %H:%M:%S%.f",
        )
        .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual.naive_utc(), expected);

        let line = r#"time="2025-12-30T21:45:58Z" level=info msg="state: {installed:false firstHost:true managementURL:}""#;
        let expected = "2025-12-30T21:45:58Z".parse::<DateTime<Utc>>().unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"time="2025-12-30T21:38:42.103385221Z" level=info msg="loading plugin" id=io.containerd.image-verifier.v1.bindir type=io.containerd.image-verifier.v1"#;
        let expected = "2025-12-30T21:38:42.103385221Z"
            .parse::<DateTime<Utc>>()
            .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"Dec 30 21:51:44.485722 isim-dev rancher-system-agent[33266]: time="2025-12-30T21:51:44Z" level=info msg="[Applyinator] Extracting image rancher/system-agent-installer-rke2:v1.34.2-rke2r1 to directory /var/lib/rancher/agent/work/20251230-215144/408628bb343c60a58fa85e402aba50bd8b1213f3aa576ce24b36c3a1dd392130_0""#;
        let expected = "2025-12-30T21:51:44Z".parse::<DateTime<Utc>>().unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"testdata/support_bundle_backup/nodes/isim-dev/logs/containerd.log:3872:2025-12-30 21:58:14.266 [INFO][52211] cni-plugin/dataplane_linux.go 508: Disabling IPv4 forwarding ContainerID="41c85156546ac63f9402d1356a4d2dc00c4b807eed439c51678d1b94fac16f7c" Namespace="default" Pod="virt-launcher-vm-00-pb825" WorkloadEndpoint="isim--dev-k8s-virt--launcher--vm--00--pb825-eth0""#;
        let expected = chrono::NaiveDateTime::parse_from_str(
            "2025-12-30 21:58:14.266",
            "%Y-%m-%d %H:%M:%S%.f",
        )
        .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual.naive_utc(), expected);

        let line = r#"time="2025-12-30T21:45:58Z" level=info msg="state: {installed:false firstHost:true managementURL:}""#;
        let expected = "2025-12-30T21:45:58Z".parse::<DateTime<Utc>>().unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"time="2025-12-30T21:38:42.103385221Z" level=info msg="loading plugin" id=io.containerd.image-verifier.v1.bindir type=io.containerd.image-verifier.v1"#;
        let expected = "2025-12-30T21:38:42.103385221Z"
            .parse::<DateTime<Utc>>()
            .unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"Dec 30 21:46:23.277593 isim-dev rancherd[1916]: time="2025-12-30T21:46:23Z" level=info msg="Writing plan file to /var/lib/rancher/rancherd/plan/plan.json""#;
        let expected = "2025-12-30T21:46:23Z".parse::<DateTime<Utc>>().unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);

        let line = r#"Dec 30 21:46:24.892053 isim-dev rke2[2067]: time="2025-12-30T21:46:24Z" level=warning msg="Unknown flag --omitStages found in config.yaml, skipping\n""#;
        let expected = "2025-12-30T21:46:24Z".parse::<DateTime<Utc>>().unwrap();
        let actual = sb_search.find_timestamp(line).unwrap().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_log_type() {
        let sbsearch = SBSearch::new("", "").unwrap();
        assert_eq!(
            sbsearch.log_type("/nodes/node1/logs/kubelet.log"),
            LogType::System
        );
        assert_eq!(
            sbsearch.log_type("/kube-system/pods/logs/kube-apiserver.log"),
            LogType::Workload
        );
    }

    #[test]
    fn test_is_zip() {
        assert!(is_zip(Path::new("testdata/support_bundle/nodes/isim-dev.zip")).unwrap());
        assert!(!is_zip(Path::new("testdata/support_bundle/metadata.yaml")).unwrap());
        assert!(is_zip(Path::new("testdata/support_bundle/nodes/noexist")).is_err());
    }
}
