use once_cell::sync::Lazy;

pub struct Doa {
    pub id: u32,
    pub category: &'static str,
    pub title_ms: &'static str,
    pub title_en: &'static str,
    pub arabic: &'static str,
    pub transliteration: &'static str,
    pub translation_ms: &'static str,
    pub translation_en: &'static str,
    pub source: &'static str,
}

pub static ALL_DOA: Lazy<Vec<Doa>> = Lazy::new(|| {
    vec![
        // === Harian (Daily) ===
        Doa {
            id: 1,
            category: "harian",
            title_ms: "Doa Sebelum Makan",
            title_en: "Before Eating",
            arabic: "بِسْمِ اللهِ وَعَلَى بَرَكَةِ اللهِ",
            transliteration: "Bismillahi wa 'ala barakatillah",
            translation_ms: "Dengan nama Allah dan dengan berkat Allah",
            translation_en: "In the name of Allah and with the blessings of Allah",
            source: "Abu Dawud",
        },
        Doa {
            id: 2,
            category: "harian",
            title_ms: "Doa Selepas Makan",
            title_en: "After Eating",
            arabic: "اَلْحَمْدُ لِلَّهِ الَّذِي أَطْعَمَنَا وَسَقَانَا وَجَعَلَنَا مُسْلِمِينَ",
            transliteration: "Alhamdulillahilladzi at'amana wa saqana wa ja'alana muslimin",
            translation_ms: "Segala puji bagi Allah yang memberi kami makan dan minum serta menjadikan kami orang Islam",
            translation_en: "Praise be to Allah who gave us food and drink and made us Muslims",
            source: "Abu Dawud, Tirmizi",
        },
        Doa {
            id: 3,
            category: "harian",
            title_ms: "Doa Masuk Rumah",
            title_en: "Entering Home",
            arabic: "بِسْمِ اللهِ وَلَجْنَا وَبِسْمِ اللهِ خَرَجْنَا وَعَلَى اللهِ رَبِّنَا تَوَكَّلْنَا",
            transliteration: "Bismillahi walajna wa bismillahi kharajna wa 'alallahi rabbina tawakkalna",
            translation_ms: "Dengan nama Allah kami masuk, dengan nama Allah kami keluar, dan kepada Allah Tuhan kami, kami bertawakkal",
            translation_en: "In the name of Allah we enter, in the name of Allah we leave, and upon Allah our Lord we rely",
            source: "Abu Dawud",
        },
        Doa {
            id: 4,
            category: "harian",
            title_ms: "Doa Keluar Rumah",
            title_en: "Leaving Home",
            arabic: "بِسْمِ اللهِ تَوَكَّلْتُ عَلَى اللهِ لَا حَوْلَ وَلَا قُوَّةَ إِلَّا بِاللهِ",
            transliteration: "Bismillahi tawakkaltu 'alallahi la hawla wa la quwwata illa billah",
            translation_ms: "Dengan nama Allah, aku bertawakkal kepada Allah. Tiada daya dan kekuatan melainkan dengan Allah",
            translation_en: "In the name of Allah, I rely on Allah. There is no power nor strength except with Allah",
            source: "Abu Dawud, Tirmizi",
        },
        // === Pagi (Morning) ===
        Doa {
            id: 5,
            category: "pagi",
            title_ms: "Zikir Pagi - Ayat Kursi",
            title_en: "Morning - Ayatul Kursi",
            arabic: "اللَّهُ لَا إِلَهَ إِلَّا هُوَ الْحَيُّ الْقَيُّومُ",
            transliteration: "Allahu la ilaha illa huwal hayyul qayyum...",
            translation_ms: "Allah, tiada tuhan selain Dia, Yang Hidup, Yang Berdiri Sendiri",
            translation_en: "Allah, there is no deity except Him, the Ever-Living, the Sustainer",
            source: "Al-Baqarah 2:255",
        },
        Doa {
            id: 6,
            category: "pagi",
            title_ms: "Zikir Pagi - Sayyidul Istighfar",
            title_en: "Morning - Master Supplication for Forgiveness",
            arabic: "اَللَّهُمَّ أَنْتَ رَبِّي لَا إِلَهَ إِلَّا أَنْتَ خَلَقْتَنِي وَأَنَا عَبْدُكَ",
            transliteration: "Allahumma anta rabbi la ilaha illa anta khalaqtani wa ana 'abduk...",
            translation_ms: "Ya Allah, Engkau Tuhanku, tiada tuhan selain Engkau, Engkau menciptakan aku dan aku hamba-Mu",
            translation_en: "O Allah, You are my Lord, there is no deity except You, You created me and I am Your servant",
            source: "Bukhari",
        },
        Doa {
            id: 7,
            category: "pagi",
            title_ms: "Zikir Pagi - Perlindungan",
            title_en: "Morning - Seeking Protection",
            arabic: "أَصْبَحْنَا وَأَصْبَحَ الْمُلْكُ لِلَّهِ وَالْحَمْدُ لِلَّهِ",
            transliteration: "Asbahna wa asbahal mulku lillahi wal hamdulillah",
            translation_ms: "Kami memasuki waktu pagi dan kerajaan adalah milik Allah, dan segala puji bagi Allah",
            translation_en: "We have entered the morning and the kingdom belongs to Allah, and praise is for Allah",
            source: "Muslim",
        },
        // === Petang (Evening) ===
        Doa {
            id: 8,
            category: "petang",
            title_ms: "Zikir Petang - Perlindungan",
            title_en: "Evening - Seeking Protection",
            arabic: "أَمْسَيْنَا وَأَمْسَى الْمُلْكُ لِلَّهِ وَالْحَمْدُ لِلَّهِ",
            transliteration: "Amsayna wa amsal mulku lillahi wal hamdulillah",
            translation_ms: "Kami memasuki waktu petang dan kerajaan adalah milik Allah, dan segala puji bagi Allah",
            translation_en: "We have entered the evening and the kingdom belongs to Allah, and praise is for Allah",
            source: "Muslim",
        },
        Doa {
            id: 9,
            category: "petang",
            title_ms: "Zikir Petang - Mohon Keselamatan",
            title_en: "Evening - Seeking Safety",
            arabic: "اَللَّهُمَّ إِنِّي أَسْأَلُكَ الْعَافِيَةَ فِي الدُّنْيَا وَالْآخِرَةِ",
            transliteration: "Allahumma inni as'alukal 'afiyah fid dunya wal akhirah",
            translation_ms: "Ya Allah, aku memohon keselamatan di dunia dan akhirat",
            translation_en: "O Allah, I ask You for well-being in this world and the Hereafter",
            source: "Abu Dawud, Ibnu Majah",
        },
        // === Solat (Prayer) ===
        Doa {
            id: 10,
            category: "solat",
            title_ms: "Doa Selepas Azan",
            title_en: "After Adhan",
            arabic: "اَللَّهُمَّ رَبَّ هَذِهِ الدَّعْوَةِ التَّامَّةِ وَالصَّلَاةِ الْقَائِمَةِ",
            transliteration: "Allahumma rabba hadhihid da'watit tammah was solatil qo'imah...",
            translation_ms: "Ya Allah, Tuhan yang memiliki seruan yang sempurna ini dan solat yang didirikan",
            translation_en: "O Allah, Lord of this perfect call and established prayer",
            source: "Bukhari",
        },
        Doa {
            id: 11,
            category: "solat",
            title_ms: "Doa Iftitah",
            title_en: "Opening Supplication in Prayer",
            arabic: "وَجَّهْتُ وَجْهِيَ لِلَّذِي فَطَرَ السَّمَاوَاتِ وَالْأَرْضَ حَنِيفًا مُسْلِمًا",
            transliteration: "Wajjahtu wajhiya lilladzi fataras samawati wal ard hanifan musliman",
            translation_ms: "Aku hadapkan wajahku kepada Tuhan yang menciptakan langit dan bumi dengan penuh keikhlasan",
            translation_en: "I have turned my face to the One who created the heavens and the earth, sincerely",
            source: "Muslim",
        },
        // === Musafir (Travel) ===
        Doa {
            id: 12,
            category: "musafir",
            title_ms: "Doa Naik Kenderaan",
            title_en: "Boarding a Vehicle",
            arabic: "سُبْحَانَ الَّذِي سَخَّرَ لَنَا هَذَا وَمَا كُنَّا لَهُ مُقْرِنِينَ وَإِنَّا إِلَى رَبِّنَا لَمُنْقَلِبُونَ",
            transliteration: "Subhanalladzi sakhkhara lana hadha wa ma kunna lahu muqrinin wa inna ila rabbina lamunqalibun",
            translation_ms: "Maha Suci Tuhan yang memudahkan ini untuk kami sedangkan kami tidak mampu menguasainya dan sesungguhnya kami akan kembali kepada Tuhan kami",
            translation_en: "Glory to Him who has subjected this to us, we could never have it by our efforts, and to our Lord we shall return",
            source: "Az-Zukhruf 43:13-14",
        },
        Doa {
            id: 13,
            category: "musafir",
            title_ms: "Doa Selamat Perjalanan",
            title_en: "Safe Journey",
            arabic: "اَللَّهُمَّ إِنَّا نَسْأَلُكَ فِي سَفَرِنَا هَذَا الْبِرَّ وَالتَّقْوَى",
            transliteration: "Allahumma inna nas'aluka fi safarina hadhal birra wat taqwa",
            translation_ms: "Ya Allah, kami memohon dalam perjalanan ini kebajikan dan ketakwaan",
            translation_en: "O Allah, we ask You in this journey for righteousness and piety",
            source: "Muslim",
        },
        // === Tidur (Sleep) ===
        Doa {
            id: 14,
            category: "tidur",
            title_ms: "Doa Sebelum Tidur",
            title_en: "Before Sleeping",
            arabic: "بِاسْمِكَ اللَّهُمَّ أَمُوتُ وَأَحْيَا",
            transliteration: "Bismika Allahumma amutu wa ahya",
            translation_ms: "Dengan nama-Mu ya Allah aku mati dan aku hidup",
            translation_en: "In Your name O Allah, I die and I live",
            source: "Bukhari",
        },
        Doa {
            id: 15,
            category: "tidur",
            title_ms: "Doa Bangun Tidur",
            title_en: "Upon Waking Up",
            arabic: "اَلْحَمْدُ لِلَّهِ الَّذِي أَحْيَانَا بَعْدَ مَا أَمَاتَنَا وَإِلَيْهِ النُّشُورُ",
            transliteration: "Alhamdulillahilladzi ahyana ba'da ma amatana wa ilaihin nushur",
            translation_ms: "Segala puji bagi Allah yang menghidupkan kami setelah mematikan kami dan kepada-Nya kami dikembalikan",
            translation_en: "Praise be to Allah who gave us life after death and to Him is the return",
            source: "Bukhari",
        },
        // === Wudhu (Ablution) ===
        Doa {
            id: 16,
            category: "wudhu",
            title_ms: "Doa Sebelum Wudhu",
            title_en: "Before Ablution",
            arabic: "بِسْمِ اللهِ الرَّحْمَنِ الرَّحِيمِ",
            transliteration: "Bismillahir rahmanir rahim",
            translation_ms: "Dengan nama Allah Yang Maha Pemurah lagi Maha Penyayang",
            translation_en: "In the name of Allah, the Most Gracious, the Most Merciful",
            source: "Abu Dawud, Tirmizi",
        },
        Doa {
            id: 17,
            category: "wudhu",
            title_ms: "Doa Selepas Wudhu",
            title_en: "After Ablution",
            arabic: "أَشْهَدُ أَنْ لَا إِلَهَ إِلَّا اللهُ وَحْدَهُ لَا شَرِيكَ لَهُ وَأَشْهَدُ أَنَّ مُحَمَّدًا عَبْدُهُ وَرَسُولُهُ",
            transliteration: "Ashhadu an la ilaha illallahu wahdahu la sharika lahu wa ashhadu anna Muhammadan 'abduhu wa rasuluh",
            translation_ms: "Aku bersaksi bahawa tiada tuhan selain Allah Yang Esa, tiada sekutu bagi-Nya, dan aku bersaksi bahawa Muhammad itu hamba-Nya dan rasul-Nya",
            translation_en: "I bear witness that there is no deity except Allah alone with no partner, and I bear witness that Muhammad is His servant and messenger",
            source: "Muslim",
        },
        // === Masjid (Mosque) ===
        Doa {
            id: 18,
            category: "masjid",
            title_ms: "Doa Masuk Masjid",
            title_en: "Entering the Mosque",
            arabic: "اَللَّهُمَّ افْتَحْ لِي أَبْوَابَ رَحْمَتِكَ",
            transliteration: "Allahummaftah li abwaba rahmatik",
            translation_ms: "Ya Allah, bukakanlah untukku pintu-pintu rahmat-Mu",
            translation_en: "O Allah, open for me the gates of Your mercy",
            source: "Muslim",
        },
        Doa {
            id: 19,
            category: "masjid",
            title_ms: "Doa Keluar Masjid",
            title_en: "Leaving the Mosque",
            arabic: "اَللَّهُمَّ إِنِّي أَسْأَلُكَ مِنْ فَضْلِكَ",
            transliteration: "Allahumma inni as'aluka min fadlik",
            translation_ms: "Ya Allah, aku memohon kepada-Mu dari kurniaan-Mu",
            translation_en: "O Allah, I ask You from Your bounty",
            source: "Muslim",
        },
        // === Makan (Food) ===
        Doa {
            id: 20,
            category: "makan",
            title_ms: "Doa Makan Lupa Bismillah",
            title_en: "Forgot Bismillah Before Eating",
            arabic: "بِسْمِ اللهِ أَوَّلَهُ وَآخِرَهُ",
            transliteration: "Bismillahi awwalahu wa akhirah",
            translation_ms: "Dengan nama Allah pada awalnya dan akhirnya",
            translation_en: "In the name of Allah at its beginning and at its end",
            source: "Abu Dawud, Tirmizi",
        },
    ]
});

pub fn get_categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("harian", "Doa Harian / Daily"),
        ("pagi", "Zikir Pagi / Morning"),
        ("petang", "Zikir Petang / Evening"),
        ("solat", "Doa Solat / Prayer"),
        ("musafir", "Doa Musafir / Travel"),
        ("makan", "Doa Makan / Food"),
        ("tidur", "Doa Tidur / Sleep"),
        ("wudhu", "Doa Wudhu / Ablution"),
        ("masjid", "Doa Masjid / Mosque"),
    ]
}

pub fn by_category(category: &str) -> Vec<&'static Doa> {
    let cat = category.to_lowercase();
    ALL_DOA.iter().filter(|d| d.category == cat).collect()
}

pub fn search_doa(query: &str) -> Vec<&'static Doa> {
    let q = query.to_lowercase();
    ALL_DOA
        .iter()
        .filter(|d| {
            d.title_ms.to_lowercase().contains(&q)
                || d.title_en.to_lowercase().contains(&q)
                || d.category.to_lowercase().contains(&q)
                || d.transliteration.to_lowercase().contains(&q)
                || d.translation_ms.to_lowercase().contains(&q)
                || d.translation_en.to_lowercase().contains(&q)
        })
        .collect()
}

pub fn random_doa() -> &'static Doa {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let idx = rng.gen_range(0..ALL_DOA.len());
    &ALL_DOA[idx]
}

pub fn get_by_id(id: u32) -> Option<&'static Doa> {
    ALL_DOA.iter().find(|d| d.id == id)
}
