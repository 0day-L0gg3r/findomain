#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

use postgres::{Connection, TlsMode};
use rand::Rng;
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    thread,
    time::Duration,
};
use trust_dns_resolver::{config::ResolverConfig, config::ResolverOpts, Resolver};

mod auth;

#[derive(Deserialize)]
struct SubdomainsCertSpotter {
    dns_names: Vec<String>,
}

#[derive(Deserialize)]
struct SubdomainsCrtsh {
    name_value: String,
}

#[allow(non_snake_case)]
struct SubdomainsDBCrtsh {
    NAME_VALUE: String,
}

#[derive(Deserialize)]
struct SubdomainsVirustotal {
    id: String,
}

#[derive(Deserialize)]
struct ResponseDataVirusTotal {
    data: Vec<SubdomainsVirustotal>,
}

#[derive(Deserialize)]
struct SubdomainsFacebook {
    domains: Vec<String>,
}

#[derive(Deserialize)]
struct ResponseDataFacebook {
    data: Vec<SubdomainsFacebook>,
}

#[derive(Deserialize)]
struct SubdomainsSpyse {
    domain: String,
}

#[derive(Deserialize)]
struct ResponseDataSpyse {
    records: Vec<SubdomainsSpyse>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct SubdomainsBufferover {
    FDNS_A: Vec<String>,
}

#[derive(Deserialize)]
struct SubdomainsThreadcrowd {
    subdomains: Vec<String>,
}

#[derive(Deserialize)]
struct SubdomainsVirustotalApikey {
    subdomains: Vec<String>,
}

lazy_static! {
    static ref RNUM: String = rand::thread_rng().gen_range(0, 10000).to_string();
    static ref CLIENT: reqwest::Client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
}

pub fn get_subdomains(target: &str, with_ip: &str, with_output: &str, file_format: &str) {
    let target = target
        .replace("www.", "")
        .replace("https://", "")
        .replace("http://", "")
        .replace("/", "");

    println!("\nTarget ==> {}\n", &target);

    let spyse_access_token = auth::get_auth_token("spyse");
    let facebook_access_token = auth::get_auth_token("facebook");
    let virustotal_access_token = auth::get_auth_token("virustotal");

    let url_api_certspotter = [
        "https://api.certspotter.com/v1/issuances?domain=",
        &target,
        "&include_subdomains=true&expand=dns_names",
    ]
    .concat();
    let url_api_virustotal = [
        "https://www.virustotal.com/ui/domains/",
        &target,
        "/subdomains?limit=40",
    ]
    .concat();
    let url_api_crtsh = ["https://crt.sh/?q=%.", &target, "&output=json"].concat();
    let crtsh_db_query = ["SELECT ci.NAME_VALUE NAME_VALUE FROM certificate_identity ci WHERE ci.NAME_TYPE = 'dNSName' AND reverse(lower(ci.NAME_VALUE)) LIKE reverse(lower('%.", &target, "'))"].concat();
    let url_api_sublist3r = ["https://api.sublist3r.com/search.php?domain=", &target].concat();
    let url_api_spyse = [
        "https://api.spyse.com/v1/subdomains?domain=",
        &target,
        "&api_token=",
        &spyse_access_token,
    ]
    .concat();
    let url_api_bufferover = ["http://dns.bufferover.run/dns?q=", &target].concat();
    let url_api_threatcrowd = [
        "https://threatcrowd.org/searchApi/v2/domain/report/?domain=",
        &target,
    ]
    .concat();
    let all_subdomains = vec![
        thread::spawn(move || get_certspotter_subdomains(&url_api_certspotter)),
        thread::spawn(move || get_crtsh_db_subdomains(&crtsh_db_query, &url_api_crtsh)),
        thread::spawn(move || get_virustotal_subdomains(&url_api_virustotal)),
        thread::spawn(move || get_sublist3r_subdomains(&url_api_sublist3r)),
        if facebook_access_token.is_empty() {
            let findomain_fb_tokens = [
                "688177841647920|RAeNYr8jwFXGH9v-IhGv4tfHMpU",
                "772592906530976|CNkO7OxM6ssQgOBLCraC_dhKE7M",
                "1004691886529013|iiUStPqcXCELcwv89-SZQSqqFNY",
                "2106186849683294|beVoPBtLp3IWjpLsnF6Mpzo1gVM",
                "2095886140707025|WkO8gTgPtwmnNZL3NQ74z92DA-k",
                "434231614102088|pLJSVc9iOqxrG6NO7DDPrlkQ1qE",
                "431009107520610|AX8VNunXMng-ainHO8Ke0sdeMJI",
                "893300687707948|KW_O07biKRaW5fpNqeAeSrMU1W8",
                "2477772448946546|BXn-h2zX6qb4WsFvtOywrNsDixo",
                "509488472952865|kONi75jYL_KQ_6J1CHPQ1MH4x_U",
            ];
            let url_api_fb = [
                "https://graph.facebook.com/certificates?query=",
                &target,
                "&fields=domains&limit=10000&access_token=",
                &findomain_fb_tokens[rand::thread_rng().gen_range(0, findomain_fb_tokens.len())],
            ]
            .concat();
            thread::spawn(move || get_facebook_subdomains(&url_api_fb))
        } else {
            let url_api_fb = [
                "https://graph.facebook.com/certificates?query=",
                &target,
                "&fields=domains&limit=10000&access_token=",
                &facebook_access_token,
            ]
            .concat();
            thread::spawn(move || get_facebook_subdomains(&url_api_fb))
        },
        thread::spawn(move || get_spyse_subdomains(&url_api_spyse)),
        thread::spawn(move || get_bufferover_subdomains(&url_api_bufferover)),
        thread::spawn(move || get_threatcrowd_subdomains(&url_api_threatcrowd)),
        if virustotal_access_token.is_empty() {
            thread::spawn(|| None)
        } else {
            let url_virustotal_apikey = [
                "https://www.virustotal.com/vtapi/v2/domain/report?apikey=",
                &virustotal_access_token,
                "&domain=",
                &target,
            ]
            .concat();
            thread::spawn(move || get_virustotal_apikey_subdomains(&url_virustotal_apikey))
        },
    ];

    let all_subdomains_vec = all_subdomains
        .into_iter()
        .map(|j| j.join().unwrap())
        .collect::<Vec<_>>();

    manage_subdomains_data(
        all_subdomains_vec
            .iter()
            .flatten()
            .flat_map(|sub| sub)
            .collect(),
        &target,
        &with_ip,
        &with_output,
        &file_format,
    );
    if with_ip == "y" && with_output == "y" {
        let filename = [&target, "_", &RNUM.to_string(), "-ip", ".", file_format].concat();
        println!(
            ">> 📁 Filename for the target {} was saved in: ./{} 😀",
            &target, &filename
        )
    } else if with_output == "y" {
        let filename = [&target, "_", &RNUM.to_string(), ".", file_format].concat();
        println!(
            ">> 📁 Filename for the target {} was saved in: ./{} 😀",
            &target, &filename
        )
    }
}

fn manage_subdomains_data(
    mut vec_subdomains: Vec<&String>,
    target: &str,
    with_ip: &str,
    with_output: &str,
    file_format: &str,
) {
    let base_target = [".", &target].concat();
    if vec_subdomains.is_empty() {
        println!(
            "\nNo subdomains were found for the target: {} ¡😭!\n",
            &target
        );
    } else {
        vec_subdomains.sort();
        vec_subdomains.dedup();
        vec_subdomains.retain(|sub| !sub.contains("*.") && sub.contains(&base_target));
        println!(
            "\nA total of {} subdomains were found for ==>  {} 👽\n",
            &vec_subdomains.len(),
            &target
        );
        for subdomain in vec_subdomains {
            if with_ip == "y" && with_output == "y" {
                let ipadress = get_ip(&subdomain);
                write_to_file(&subdomain, &target, &ipadress, &file_format, &with_ip);
                println!("{},{}", &subdomain, &ipadress);
            } else if with_ip == "y" {
                let ipadress = get_ip(&subdomain);
                println!("{},{}", &subdomain, &ipadress);
            } else if with_output == "y" {
                let ipadress = "";
                write_to_file(&subdomain, &target, &ipadress, &file_format, &with_ip);
                println!("{}", &subdomain);
            } else {
                println!("{}", &subdomain);
            }
        }
        println!("\nGood luck Hax0r 💀!\n");
    }
}

fn get_certspotter_subdomains(url_api_certspotter: &str) -> Option<Vec<String>> {
    println!("Searching in the CertSpotter API... 🔍");
    match CLIENT.get(url_api_certspotter).send() {
        Ok(mut data_certspotter) => match data_certspotter.json::<Vec<SubdomainsCertSpotter>>() {
            Ok(domains_certspotter) => Some(
                domains_certspotter
                    .into_iter()
                    .flat_map(|sub| sub.dns_names.into_iter())
                    .collect(),
            ),
            Err(e) => {
                check_json_errors(e, "CertSpotter");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "CertSpotter");
            None
        }
    }
}

fn get_crtsh_subdomains(url_api_crtsh: &str) -> Option<Vec<String>> {
    println!("Searching in the Crtsh API... 🔍");
    match CLIENT.get(url_api_crtsh).send() {
        Ok(mut data_crtsh) => match data_crtsh.json::<Vec<SubdomainsCrtsh>>() {
            Ok(domains_crtsh) => Some(
                domains_crtsh
                    .into_iter()
                    .map(|sub| sub.name_value)
                    .collect(),
            ),
            Err(e) => {
                check_json_errors(e, "Crtsh");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Crtsh");
            None
        }
    }
}

fn get_crtsh_db_subdomains(crtsh_db_query: &str, url_api_crtsh: &str) -> Option<Vec<String>> {
    println!("Searching in the Crtsh database... 🔍");
    match Connection::connect("postgres://guest@crt.sh:5432/certwatch", TlsMode::None) {
        Ok(crtsh_db_client) => match crtsh_db_client.query(&crtsh_db_query, &[]) {
            Ok(crtsh_db_subdomains) => Some(
                crtsh_db_subdomains
                    .iter()
                    .map(|row| {
                        let subdomain = SubdomainsDBCrtsh {
                            NAME_VALUE: row.get("NAME_VALUE"),
                        };
                        subdomain.NAME_VALUE
                    })
                    .collect(),
            ),
            Err(e) => {
                println!(
                    "A error ❌ has occurred while querying the Crtsh database. Error: {}. Trying the API method...",
                    e.description()
                );
                get_crtsh_subdomains(&url_api_crtsh)
            }
        },
        Err(e) => {
            println!(
                "A error ❌ has occurred while connecting to the Crtsh database. Error: {}. Trying the API method...",
                e.description()
            );
            get_crtsh_subdomains(&url_api_crtsh)
        }
    }
}

fn get_virustotal_subdomains(url_api_virustotal: &str) -> Option<Vec<String>> {
    println!("Searching in the Virustotal API... 🔍");
    match CLIENT.get(url_api_virustotal).send() {
        Ok(mut data_virustotal) => match data_virustotal.json::<ResponseDataVirusTotal>() {
            Ok(virustotal_json) => {
                let domains_virustotal = virustotal_json.data;
                Some(domains_virustotal.into_iter().map(|sub| sub.id).collect())
            }
            Err(e) => {
                check_json_errors(e, "Virustotal");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Virustotal");
            None
        }
    }
}

fn get_sublist3r_subdomains(url_api_sublist3r: &str) -> Option<Vec<String>> {
    println!("Searching in the Sublist3r API... 🔍");
    match CLIENT.get(url_api_sublist3r).send() {
        Ok(mut data_sublist3r) => match data_sublist3r.json::<Vec<String>>() {
            Ok(domains_sublist3r) => Some(domains_sublist3r),
            Err(e) => {
                check_json_errors(e, "Sublist3r");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Sublist3r");
            None
        }
    }
}

fn get_facebook_subdomains(url_api_fb: &str) -> Option<Vec<String>> {
    println!("Searching in the Facebook API... 🔍");
    match CLIENT.get(url_api_fb).send() {
        Ok(mut data_fb) => match data_fb.json::<ResponseDataFacebook>() {
            Ok(fb_json) => Some(
                fb_json
                    .data
                    .into_iter()
                    .flat_map(|sub| sub.domains.into_iter())
                    .collect(),
            ),
            Err(e) => {
                check_json_errors(e, "Facebook");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Facebook");
            None
        }
    }
}

fn get_spyse_subdomains(url_api_spyse: &str) -> Option<Vec<String>> {
    println!("Searching in the Spyse API... 🔍");
    match CLIENT.get(url_api_spyse).send() {
        Ok(mut data_spyse) => match data_spyse.json::<ResponseDataSpyse>() {
            Ok(spyse_json) => {
                let domains_spyse = spyse_json.records;
                Some(domains_spyse.into_iter().map(|sub| sub.domain).collect())
            }
            Err(e) => {
                check_json_errors(e, "Spyse");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Spyse");
            None
        }
    }
}

fn get_bufferover_subdomains(url_api_bufferover: &str) -> Option<Vec<String>> {
    println!("Searching in the Bufferover API... 🔍");
    match CLIENT.get(url_api_bufferover).send() {
        Ok(mut data_bufferover) => match data_bufferover.json::<SubdomainsBufferover>() {
            Ok(bufferover_json) => Some(
                bufferover_json
                    .FDNS_A
                    .iter()
                    .map(|sub| sub.split(","))
                    .flatten()
                    .map(str::to_owned)
                    .collect(),
            ),
            Err(e) => {
                check_json_errors(e, "Bufferover");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Bufferover");
            None
        }
    }
}

fn get_threatcrowd_subdomains(url_api_threatcrowd: &str) -> Option<Vec<String>> {
    println!("Searching in the Threadcrowd API... 🔍");
    match CLIENT.get(url_api_threatcrowd).send() {
        Ok(mut data_threatcrowd) => match data_threatcrowd.json::<SubdomainsThreadcrowd>() {
            Ok(threatcrowd_json) => Some(
                threatcrowd_json
                    .subdomains
                    .into_iter()
                    .map(|sub| sub)
                    .collect(),
            ),
            Err(e) => {
                check_json_errors(e, "Threadcrowd");
                None
            }
        },
        Err(e) => {
            check_request_errors(e, "Threadcrowd");
            None
        }
    }
}

fn get_virustotal_apikey_subdomains(url_virustotal_apikey: &str) -> Option<Vec<String>> {
    println!("Searching in the Virustotal API using apikey... 🔍");
    match CLIENT.get(url_virustotal_apikey).send() {
        Ok(mut data_virustotal_apikey) => {
            match data_virustotal_apikey.json::<SubdomainsVirustotalApikey>() {
                Ok(virustotal_apikey_json) => Some(
                    virustotal_apikey_json
                        .subdomains
                        .into_iter()
                        .map(|sub| sub)
                        .collect(),
                ),
                Err(e) => {
                    check_json_errors(e, "Virustotal API using apikey");
                    None
                }
            }
        }
        Err(e) => {
            check_request_errors(e, "Virustotal API using apikey");
            None
        }
    }
}

fn check_request_errors(error: reqwest::Error, api: &str) {
    if error.is_timeout() {
        println!(
            "A timeout ⏳ error has occurred while processing the request in the {} API. Error description: {}\n",
            &api, &error.description())
    } else if error.is_redirect() {
        println!(
            "A redirect ↪️  was found while processing the {} API. Error description: {}\n",
            &api,
            &error.description()
        )
    } else if error.is_client_error() {
        println!(
            "A client error 🧑❌ has occurred sending the request to the {} API. Error description: {}\n",
            &api,
            &error.description()
        )
    } else if error.is_server_error() {
        println!(
            "A server error 🖥️❌ has occurred sending the request to the {} API. Error description: {}\n",
            &api,
            &error.description()
        )
    } else {
        println!(
            "An error ❌ has occurred while procesing the request in the {} API. Error description: {}\n",
            &api,
            &error.description()
        )
    }
}

fn check_json_errors(error: reqwest::Error, api: &str) {
    println!("An error ❌ has occurred while parsing the JSON obtained from the {} API. Error description: {}.\n", &api, error.description())
}

pub fn read_from_file(file: &str, with_ip: &str, with_output: &str, file_format: &str) {
    if let Ok(f) = File::open(&file) {
        let f = BufReader::new(f);
        for line in f.lines() {
            get_subdomains(
                &line.unwrap().to_string(),
                &with_ip,
                &with_output,
                &file_format,
            )
        }
    } else {
        println!(
            "Error: can't open file 📁 {}, please check the filename and try again.",
            &file
        );
    }
}

fn write_to_file(data: &str, target: &str, subdomain_ip: &str, file_format: &str, with_ip: &str) {
    if with_ip == "y" {
        let data = &[data, ",", subdomain_ip, "\n"].concat();
        let with_ip = "-ip";
        let filename = &[target, "_", &RNUM, with_ip, ".", file_format].concat();
        if Path::new(&filename).exists() {
            let mut output_file = OpenOptions::new()
                .append(true)
                .open(&filename)
                .expect("Can't open file.");
            output_file
                .write_all(&data.as_bytes())
                .expect("Failed writing to file.");
        } else {
            File::create(&filename).expect("Failed to create file.");
            let mut output_file = OpenOptions::new()
                .append(true)
                .open(&filename)
                .expect("Can't open file.");
            output_file
                .write_all("subdomain,ip\n".as_bytes())
                .expect("Failed writing to file.");
            output_file
                .write_all(&data.as_bytes())
                .expect("Failed writing to file.");
        }
    } else {
        let data = &[data, "\n"].concat();
        let filename = &[target, "_", &RNUM, ".", file_format].concat();
        if Path::new(&filename).exists() {
            let mut output_file = OpenOptions::new()
                .append(true)
                .open(&filename)
                .expect("Can't open file.");
            output_file
                .write_all(&data.as_bytes())
                .expect("Failed writing to file.");
        } else {
            File::create(&filename).expect("Failed to create file.");
            let mut output_file = OpenOptions::new()
                .append(true)
                .open(&filename)
                .expect("Can't open file.");
            output_file
                .write_all("subdomain\n".as_bytes())
                .expect("Failed writing to file.");
            output_file
                .write_all(&data.as_bytes())
                .expect("Failed writing to file.");
        }
    }
}

fn get_ip(domain: &str) -> String {
    let resolver = get_resolver();
    match resolver.lookup_ip(&domain) {
        Ok(ip_address) => ip_address
            .iter()
            .next()
            .expect("An error has occurred getting the IP address.")
            .to_string(),
        Err(_) => String::from("No IP address found"),
    }
}

fn get_resolver() -> Resolver {
    match Resolver::from_system_conf() {
        Ok(system_resolver) => system_resolver,
        Err(_) => match Resolver::new(ResolverConfig::quad9(), ResolverOpts::default()) {
            Ok(quad9_resolver) => quad9_resolver,
            Err(_) => match Resolver::new(ResolverConfig::cloudflare(), ResolverOpts::default()) {
                Ok(cloudflare_resolver) => cloudflare_resolver,
                Err(_) => {
                    let defaul_resolver =
                        Resolver::new(ResolverConfig::default(), ResolverOpts::default()).unwrap();
                    defaul_resolver
                }
            },
        },
    }
}
