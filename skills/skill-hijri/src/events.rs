pub struct IslamicEvent {
    pub month: u32,
    pub day: u32,
    pub name_ms: &'static str,
    pub name_en: &'static str,
}

pub fn get_events() -> Vec<IslamicEvent> {
    vec![
        IslamicEvent {
            month: 1,
            day: 1,
            name_ms: "Awal Muharram",
            name_en: "Islamic New Year",
        },
        IslamicEvent {
            month: 1,
            day: 10,
            name_ms: "Hari Asyura",
            name_en: "Day of Ashura",
        },
        IslamicEvent {
            month: 3,
            day: 12,
            name_ms: "Mawlidur Rasul",
            name_en: "Prophet's Birthday",
        },
        IslamicEvent {
            month: 7,
            day: 27,
            name_ms: "Israk & Mikraj",
            name_en: "Isra and Mi'raj",
        },
        IslamicEvent {
            month: 8,
            day: 15,
            name_ms: "Nisfu Sya'ban",
            name_en: "Mid-Sha'ban",
        },
        IslamicEvent {
            month: 9,
            day: 1,
            name_ms: "Ramadan Bermula",
            name_en: "Start of Ramadan",
        },
        IslamicEvent {
            month: 9,
            day: 17,
            name_ms: "Nuzul Al-Quran",
            name_en: "Revelation of Quran",
        },
        IslamicEvent {
            month: 9,
            day: 27,
            name_ms: "Lailatul Qadr",
            name_en: "Night of Power",
        },
        IslamicEvent {
            month: 10,
            day: 1,
            name_ms: "Hari Raya Aidilfitri",
            name_en: "Eid al-Fitr",
        },
        IslamicEvent {
            month: 12,
            day: 9,
            name_ms: "Hari Arafah",
            name_en: "Day of Arafah",
        },
        IslamicEvent {
            month: 12,
            day: 10,
            name_ms: "Hari Raya Haji",
            name_en: "Eid al-Adha",
        },
    ]
}
