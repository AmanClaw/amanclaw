pub struct Zone {
    pub code: &'static str,
    pub state: &'static str,
    pub areas: &'static str,
}

pub fn get_all_zones() -> Vec<Zone> {
    vec![
        // Johor
        Zone {
            code: "JHR01",
            state: "Johor",
            areas: "Pulau Aur, Pemanggil",
        },
        Zone {
            code: "JHR02",
            state: "Johor",
            areas: "Johor Bahru, Kota Tinggi, Mersing, Kulai",
        },
        Zone {
            code: "JHR03",
            state: "Johor",
            areas: "Kluang, Pontian",
        },
        Zone {
            code: "JHR04",
            state: "Johor",
            areas: "Batu Pahat, Muar, Segamat, Gemas",
        },
        // Kedah
        Zone {
            code: "KDH01",
            state: "Kedah",
            areas: "Kota Setar, Kubang Pasu, Pokok Sena",
        },
        Zone {
            code: "KDH02",
            state: "Kedah",
            areas: "Kuala Muda, Yan, Pendang",
        },
        Zone {
            code: "KDH03",
            state: "Kedah",
            areas: "Padang Terap, Sik",
        },
        Zone {
            code: "KDH04",
            state: "Kedah",
            areas: "Baling",
        },
        Zone {
            code: "KDH05",
            state: "Kedah",
            areas: "Kulim, Bandar Baharu",
        },
        Zone {
            code: "KDH06",
            state: "Kedah",
            areas: "Langkawi",
        },
        Zone {
            code: "KDH07",
            state: "Kedah",
            areas: "Gunung Jerai",
        },
        // Kelantan
        Zone {
            code: "KTN01",
            state: "Kelantan",
            areas: "Kota Bharu, Bachok, Pasir Puteh, Tumpat, Pasir Mas, Tanah Merah, Machang, Kuala Krai, Mukim Chiku",
        },
        Zone {
            code: "KTN02",
            state: "Kelantan",
            areas: "Gua Musang (Daerah Galas, Bertam), Jeli",
        },
        // Melaka
        Zone {
            code: "MLK01",
            state: "Melaka",
            areas: "Seluruh Negeri Melaka",
        },
        // Negeri Sembilan
        Zone {
            code: "NGS01",
            state: "Negeri Sembilan",
            areas: "Tampin, Jempol",
        },
        Zone {
            code: "NGS02",
            state: "Negeri Sembilan",
            areas: "Port Dickson, Seremban, Kuala Pilah, Jelebu, Rembau",
        },
        // Pahang
        Zone {
            code: "PHG01",
            state: "Pahang",
            areas: "Pulau Tioman",
        },
        Zone {
            code: "PHG02",
            state: "Pahang",
            areas: "Kuantan, Pekan, Rompin, Muadzam Shah",
        },
        Zone {
            code: "PHG03",
            state: "Pahang",
            areas: "Jerantut, Temerloh, Maran, Bera, Chenor, Jengka",
        },
        Zone {
            code: "PHG04",
            state: "Pahang",
            areas: "Bentong, Lipis, Raub",
        },
        Zone {
            code: "PHG05",
            state: "Pahang",
            areas: "Genting Highlands, Cameron Highlands",
        },
        // Perlis
        Zone {
            code: "PLS01",
            state: "Perlis",
            areas: "Seluruh Negeri Perlis",
        },
        // Pulau Pinang
        Zone {
            code: "PNG01",
            state: "Pulau Pinang",
            areas: "Seluruh Negeri Pulau Pinang",
        },
        // Perak
        Zone {
            code: "PRK01",
            state: "Perak",
            areas: "Tapah, Slim River, Tanjung Malim",
        },
        Zone {
            code: "PRK02",
            state: "Perak",
            areas: "Kuala Kangsar, Sg. Siput, Ipoh, Kampar, Batu Gajah, Hulu Perak",
        },
        Zone {
            code: "PRK03",
            state: "Perak",
            areas: "Lenggong, Pengkalan Hulu, Grik",
        },
        Zone {
            code: "PRK04",
            state: "Perak",
            areas: "Temengor, Belum",
        },
        Zone {
            code: "PRK05",
            state: "Perak",
            areas: "Teluk Intan, Bagan Datoh, Kg. Gajah, Sri Iskandar, Beruas, Parit, Lumut, Sitiawan, Pulau Pangkor",
        },
        Zone {
            code: "PRK06",
            state: "Perak",
            areas: "Selama, Taiping, Bagan Serai, Parit Buntar",
        },
        Zone {
            code: "PRK07",
            state: "Perak",
            areas: "Bukit Larut",
        },
        // Sabah
        Zone {
            code: "SBH01",
            state: "Sabah",
            areas: "Sandakan, Tungku, Sungai Imam",
        },
        Zone {
            code: "SBH02",
            state: "Sabah",
            areas: "Pinangah, Keningau, Tambunan, Nabawan",
        },
        Zone {
            code: "SBH03",
            state: "Sabah",
            areas: "Lahad Datu, Silabukan, Kunak, Semporna, Tungku, Tawau",
        },
        Zone {
            code: "SBH04",
            state: "Sabah",
            areas: "Pensiangan, Sapulut",
        },
        Zone {
            code: "SBH05",
            state: "Sabah",
            areas: "Papar, Ranau, Kota Marudu, Tuaran, Penampang, Kota Belud",
        },
        Zone {
            code: "SBH06",
            state: "Sabah",
            areas: "Gunung Kinabalu",
        },
        Zone {
            code: "SBH07",
            state: "Sabah",
            areas: "Kota Kinabalu, Putatan, Menggatal",
        },
        Zone {
            code: "SBH08",
            state: "Sabah",
            areas: "Kudat, Pitas, Pulau Banggi",
        },
        Zone {
            code: "SBH09",
            state: "Sabah",
            areas: "Beaufort, Kuala Penyu, Sipitang, Tenom, Long Pasia",
        },
        // Sarawak
        Zone {
            code: "SWK01",
            state: "Sarawak",
            areas: "Limbang, Sundar, Trusan",
        },
        Zone {
            code: "SWK02",
            state: "Sarawak",
            areas: "Miri, Niah, Bekenu, Sibuti, Marudi",
        },
        Zone {
            code: "SWK03",
            state: "Sarawak",
            areas: "Tatau, Belaga, Kapit, Sebauh, Song, Bintulu",
        },
        Zone {
            code: "SWK04",
            state: "Sarawak",
            areas: "Sibu, Kanowit, Dalat, Mukah, Igan, Oya, Balingian, Julau, Daro, Sarikei",
        },
        Zone {
            code: "SWK05",
            state: "Sarawak",
            areas: "Samarahan, Sri Aman, Betong, Lubok Antu, Kabong, Lingga, Engkilili, Pusa",
        },
        Zone {
            code: "SWK06",
            state: "Sarawak",
            areas: "Kuching, Bau, Lundu, Sematan",
        },
        Zone {
            code: "SWK07",
            state: "Sarawak",
            areas: "Lawas, Merapok, Trusan",
        },
        Zone {
            code: "SWK08",
            state: "Sarawak",
            areas: "Saratok, Roban, Debak, Serembu",
        },
        Zone {
            code: "SWK09",
            state: "Sarawak",
            areas: "Serian",
        },
        // Selangor
        Zone {
            code: "SGR01",
            state: "Selangor",
            areas: "Gombak, Petaling, Sepang, Hulu Langat, Hulu Selangor, Shah Alam",
        },
        Zone {
            code: "SGR02",
            state: "Selangor",
            areas: "Kuala Selangor, Sabak Bernam",
        },
        Zone {
            code: "SGR03",
            state: "Selangor",
            areas: "Klang, Kuala Langat",
        },
        // Terengganu
        Zone {
            code: "TRG01",
            state: "Terengganu",
            areas: "Kuala Terengganu, Marang, Kuala Nerus",
        },
        Zone {
            code: "TRG02",
            state: "Terengganu",
            areas: "Besut, Setiu",
        },
        Zone {
            code: "TRG03",
            state: "Terengganu",
            areas: "Hulu Terengganu",
        },
        Zone {
            code: "TRG04",
            state: "Terengganu",
            areas: "Dungun, Kemaman",
        },
        // WP
        Zone {
            code: "WLY01",
            state: "WP Kuala Lumpur",
            areas: "Kuala Lumpur, Putrajaya",
        },
        Zone {
            code: "WLY02",
            state: "WP Labuan",
            areas: "Labuan",
        },
    ]
}

pub fn find_zone(code: &str) -> Option<Zone> {
    get_all_zones()
        .into_iter()
        .find(|z| z.code.eq_ignore_ascii_case(code))
}

pub fn zones_by_state(state: &str) -> Vec<Zone> {
    get_all_zones()
        .into_iter()
        .filter(|z| z.state.eq_ignore_ascii_case(state))
        .collect()
}

pub fn get_states() -> Vec<&'static str> {
    vec![
        "Johor",
        "Kedah",
        "Kelantan",
        "Melaka",
        "Negeri Sembilan",
        "Pahang",
        "Perak",
        "Perlis",
        "Pulau Pinang",
        "Sabah",
        "Sarawak",
        "Selangor",
        "Terengganu",
        "WP Kuala Lumpur",
        "WP Labuan",
    ]
}
