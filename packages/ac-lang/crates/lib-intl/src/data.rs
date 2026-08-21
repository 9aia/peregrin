use crate::plural::PluralRule;

/// Static locale data entry.
pub struct LocaleData {
    pub lang: &'static str,
    pub region: &'static str,
    /// Plural rules for this language.
    pub plural_rule: PluralRule,
    /// Decimal separator (e.g. "." for en, "," for pt-BR).
    pub decimal_sep: &'static str,
    /// Thousands separator (e.g. "," for en, "." for pt-BR).
    pub thousands_sep: &'static str,
    /// Duration unit words: (day, days), (hour, hours), (minute, minutes), (second, seconds).
    pub dur_units: DurUnits,
    /// Relative time templates: past/future patterns.
    /// past[0] = singular, past[1] = plural, future[0] = singular, future[1] = plural
    /// For each unit: (day, hour, minute, second).
    pub rel_past: RelTime,
    pub rel_future: RelTime,
    /// Ordinal suffixes: (1st, 2nd, 3rd, nth) for English-like.
    /// For non-English, this may be empty strings (ordinal not used).
    pub ordinal_suffixes: OrdinalSuffixes,
    /// List formatting: (start, middle, end, pair) patterns.
    /// e.g. English: ("", ", ", ", and ", " and ")
    pub list_conj: ListPatterns,
    /// "now" string for relative_time when delta is ~0.
    pub now_str: &'static str,
}

pub struct DurUnits {
    pub day_singular: &'static str,
    pub day_plural: &'static str,
    pub hour_singular: &'static str,
    pub hour_plural: &'static str,
    pub minute_singular: &'static str,
    pub minute_plural: &'static str,
    pub second_singular: &'static str,
    pub second_plural: &'static str,
}

pub struct RelTime {
    pub day_singular: &'static str,
    pub day_plural: &'static str,
    pub hour_singular: &'static str,
    pub hour_plural: &'static str,
    pub minute_singular: &'static str,
    pub minute_plural: &'static str,
    pub second_singular: &'static str,
    pub second_plural: &'static str,
}

pub struct OrdinalSuffixes {
    pub one: &'static str,
    pub two: &'static str,
    pub few: &'static str,
    pub other: &'static str,
}

pub struct ListPatterns {
    pub start: &'static str,
    pub middle: &'static str,
    pub end: &'static str,
    pub pair: &'static str,
}

// ── Static locale data table ─────────────────────────────────────

static EN_DATA: LocaleData = LocaleData {
    lang: "en",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ".",
    thousands_sep: ",",
    dur_units: DurUnits {
        day_singular: "day",
        day_plural: "days",
        hour_singular: "hour",
        hour_plural: "hours",
        minute_singular: "minute",
        minute_plural: "minutes",
        second_singular: "second",
        second_plural: "seconds",
    },
    rel_past: RelTime {
        day_singular: "1 day ago",
        day_plural: "{} days ago",
        hour_singular: "1 hour ago",
        hour_plural: "{} hours ago",
        minute_singular: "1 minute ago",
        minute_plural: "{} minutes ago",
        second_singular: "1 second ago",
        second_plural: "{} seconds ago",
    },
    rel_future: RelTime {
        day_singular: "in 1 day",
        day_plural: "in {} days",
        hour_singular: "in 1 hour",
        hour_plural: "in {} hours",
        minute_singular: "in 1 minute",
        minute_plural: "in {} minutes",
        second_singular: "in 1 second",
        second_plural: "in {} seconds",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "st",
        two: "nd",
        few: "rd",
        other: "th",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: ", and ",
        pair: " and ",
    },
    now_str: "now",
};

static PT_DATA: LocaleData = LocaleData {
    lang: "pt",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: ".",
    dur_units: DurUnits {
        day_singular: "dia",
        day_plural: "dias",
        hour_singular: "hora",
        hour_plural: "horas",
        minute_singular: "minuto",
        minute_plural: "minutos",
        second_singular: "segundo",
        second_plural: "segundos",
    },
    rel_past: RelTime {
        day_singular: "há 1 dia",
        day_plural: "há {} dias",
        hour_singular: "há 1 hora",
        hour_plural: "há {} horas",
        minute_singular: "há 1 minuto",
        minute_plural: "há {} minutos",
        second_singular: "há 1 segundo",
        second_plural: "há {} segundos",
    },
    rel_future: RelTime {
        day_singular: "em 1 dia",
        day_plural: "em {} dias",
        hour_singular: "em 1 hora",
        hour_plural: "em {} horas",
        minute_singular: "em 1 minuto",
        minute_plural: "em {} minutos",
        second_singular: "em 1 segundo",
        second_plural: "em {} segundos",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "º",
        two: "º",
        few: "º",
        other: "º",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " e ",
        pair: " e ",
    },
    now_str: "agora",
};

static ES_DATA: LocaleData = LocaleData {
    lang: "es",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: ".",
    dur_units: DurUnits {
        day_singular: "día",
        day_plural: "días",
        hour_singular: "hora",
        hour_plural: "horas",
        minute_singular: "minuto",
        minute_plural: "minutos",
        second_singular: "segundo",
        second_plural: "segundos",
    },
    rel_past: RelTime {
        day_singular: "hace 1 día",
        day_plural: "hace {} días",
        hour_singular: "hace 1 hora",
        hour_plural: "hace {} horas",
        minute_singular: "hace 1 minuto",
        minute_plural: "hace {} minutos",
        second_singular: "hace 1 segundo",
        second_plural: "hace {} segundos",
    },
    rel_future: RelTime {
        day_singular: "en 1 día",
        day_plural: "en {} días",
        hour_singular: "en 1 hora",
        hour_plural: "en {} horas",
        minute_singular: "en 1 minuto",
        minute_plural: "en {} minutos",
        second_singular: "en 1 segundo",
        second_plural: "en {} segundos",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "º",
        two: "º",
        few: "º",
        other: "º",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " y ",
        pair: " y ",
    },
    now_str: "ahora",
};

static FR_DATA: LocaleData = LocaleData {
    lang: "fr",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: " ",
    dur_units: DurUnits {
        day_singular: "jour",
        day_plural: "jours",
        hour_singular: "heure",
        hour_plural: "heures",
        minute_singular: "minute",
        minute_plural: "minutes",
        second_singular: "seconde",
        second_plural: "secondes",
    },
    rel_past: RelTime {
        day_singular: "il y a 1 jour",
        day_plural: "il y a {} jours",
        hour_singular: "il y a 1 heure",
        hour_plural: "il y a {} heures",
        minute_singular: "il y a 1 minute",
        minute_plural: "il y a {} minutes",
        second_singular: "il y a 1 seconde",
        second_plural: "il y a {} secondes",
    },
    rel_future: RelTime {
        day_singular: "dans 1 jour",
        day_plural: "dans {} jours",
        hour_singular: "dans 1 heure",
        hour_plural: "dans {} heures",
        minute_singular: "dans 1 minute",
        minute_plural: "dans {} minutes",
        second_singular: "dans 1 seconde",
        second_plural: "dans {} secondes",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "er",
        two: "e",
        few: "e",
        other: "e",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " et ",
        pair: " et ",
    },
    now_str: "maintenant",
};

static DE_DATA: LocaleData = LocaleData {
    lang: "de",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: ".",
    dur_units: DurUnits {
        day_singular: "Tag",
        day_plural: "Tage",
        hour_singular: "Stunde",
        hour_plural: "Stunden",
        minute_singular: "Minute",
        minute_plural: "Minuten",
        second_singular: "Sekunde",
        second_plural: "Sekunden",
    },
    rel_past: RelTime {
        day_singular: "vor 1 Tag",
        day_plural: "vor {} Tagen",
        hour_singular: "vor 1 Stunde",
        hour_plural: "vor {} Stunden",
        minute_singular: "vor 1 Minute",
        minute_plural: "vor {} Minuten",
        second_singular: "vor 1 Sekunde",
        second_plural: "vor {} Sekunden",
    },
    rel_future: RelTime {
        day_singular: "in 1 Tag",
        day_plural: "in {} Tagen",
        hour_singular: "in 1 Stunde",
        hour_plural: "in {} Stunden",
        minute_singular: "in 1 Minute",
        minute_plural: "in {} Minuten",
        second_singular: "in 1 Sekunde",
        second_plural: "in {} Sekunden",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: ".",
        two: ".",
        few: ".",
        other: ".",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " und ",
        pair: " und ",
    },
    now_str: "jetzt",
};

static AR_DATA: LocaleData = LocaleData {
    lang: "ar",
    region: "",
    plural_rule: PluralRule::arabic(),
    decimal_sep: "٫",
    thousands_sep: "٬",
    dur_units: DurUnits {
        day_singular: "يوم",
        day_plural: "أيام",
        hour_singular: "ساعة",
        hour_plural: "ساعات",
        minute_singular: "دقيقة",
        minute_plural: "دقائق",
        second_singular: "ثانية",
        second_plural: "ثوان",
    },
    rel_past: RelTime {
        day_singular: "منذ يوم واحد",
        day_plural: "منذ {} أيام",
        hour_singular: "منذ ساعة واحدة",
        hour_plural: "منذ {} ساعات",
        minute_singular: "منذ دقيقة واحدة",
        minute_plural: "منذ {} دقائق",
        second_singular: "منذ ثانية واحدة",
        second_plural: "منذ {} ثوان",
    },
    rel_future: RelTime {
        day_singular: "خلال يوم واحد",
        day_plural: "خلال {} أيام",
        hour_singular: "خلال ساعة واحدة",
        hour_plural: "خلال {} ساعات",
        minute_singular: "خلال دقيقة واحدة",
        minute_plural: "خلال {} دقائق",
        second_singular: "خلال ثانية واحدة",
        second_plural: "خلال {} ثوان",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: ".",
        two: ".",
        few: ".",
        other: ".",
    },
    list_conj: ListPatterns {
        start: "",
        middle: "، ",
        end: " و",
        pair: " و",
    },
    now_str: "الآن",
};

static RU_DATA: LocaleData = LocaleData {
    lang: "ru",
    region: "",
    plural_rule: PluralRule::east_slavic(),
    decimal_sep: ",",
    thousands_sep: " ",
    dur_units: DurUnits {
        day_singular: "день",
        day_plural: "дней",
        hour_singular: "час",
        hour_plural: "часов",
        minute_singular: "минута",
        minute_plural: "минут",
        second_singular: "секунда",
        second_plural: "секунд",
    },
    rel_past: RelTime {
        day_singular: "1 день назад",
        day_plural: "{} дней назад",
        hour_singular: "1 час назад",
        hour_plural: "{} часов назад",
        minute_singular: "1 минуту назад",
        minute_plural: "{} минут назад",
        second_singular: "1 секунду назад",
        second_plural: "{} секунд назад",
    },
    rel_future: RelTime {
        day_singular: "через 1 день",
        day_plural: "через {} дней",
        hour_singular: "через 1 час",
        hour_plural: "через {} часов",
        minute_singular: "через 1 минуту",
        minute_plural: "через {} минут",
        second_singular: "через 1 секунду",
        second_plural: "через {} секунд",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: ".",
        two: ".",
        few: ".",
        other: ".",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " и ",
        pair: " и ",
    },
    now_str: "сейчас",
};

static JA_DATA: LocaleData = LocaleData {
    lang: "ja",
    region: "",
    plural_rule: PluralRule::other_only(),
    decimal_sep: ".",
    thousands_sep: ",",
    dur_units: DurUnits {
        day_singular: "日",
        day_plural: "日",
        hour_singular: "時間",
        hour_plural: "時間",
        minute_singular: "分",
        minute_plural: "分",
        second_singular: "秒",
        second_plural: "秒",
    },
    rel_past: RelTime {
        day_singular: "1日前",
        day_plural: "{}日前",
        hour_singular: "1時間前",
        hour_plural: "{}時間前",
        minute_singular: "1分前",
        minute_plural: "{}分前",
        second_singular: "1秒前",
        second_plural: "{}秒前",
    },
    rel_future: RelTime {
        day_singular: "1日後",
        day_plural: "{}日後",
        hour_singular: "1時間後",
        hour_plural: "{}時間後",
        minute_singular: "1分後",
        minute_plural: "{}分後",
        second_singular: "1秒後",
        second_plural: "{}秒後",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "",
        two: "",
        few: "",
        other: "",
    },
    list_conj: ListPatterns {
        start: "",
        middle: "、",
        end: "、",
        pair: "と",
    },
    now_str: "今",
};

static ZH_DATA: LocaleData = LocaleData {
    lang: "zh",
    region: "",
    plural_rule: PluralRule::other_only(),
    decimal_sep: ".",
    thousands_sep: ",",
    dur_units: DurUnits {
        day_singular: "天",
        day_plural: "天",
        hour_singular: "小时",
        hour_plural: "小时",
        minute_singular: "分钟",
        minute_plural: "分钟",
        second_singular: "秒",
        second_plural: "秒",
    },
    rel_past: RelTime {
        day_singular: "1天前",
        day_plural: "{}天前",
        hour_singular: "1小时前",
        hour_plural: "{}小时前",
        minute_singular: "1分钟前",
        minute_plural: "{}分钟前",
        second_singular: "1秒前",
        second_plural: "{}秒前",
    },
    rel_future: RelTime {
        day_singular: "1天后",
        day_plural: "{}天后",
        hour_singular: "1小时后",
        hour_plural: "{}小时后",
        minute_singular: "1分钟后",
        minute_plural: "{}分钟后",
        second_singular: "1秒后",
        second_plural: "{}秒后",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "",
        two: "",
        few: "",
        other: "",
    },
    list_conj: ListPatterns {
        start: "",
        middle: "、",
        end: "、",
        pair: "和",
    },
    now_str: "现在",
};

static IT_DATA: LocaleData = LocaleData {
    lang: "it",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: ".",
    dur_units: DurUnits {
        day_singular: "giorno",
        day_plural: "giorni",
        hour_singular: "ora",
        hour_plural: "ore",
        minute_singular: "minuto",
        minute_plural: "minuti",
        second_singular: "secondo",
        second_plural: "secondi",
    },
    rel_past: RelTime {
        day_singular: "1 giorno fa",
        day_plural: "{} giorni fa",
        hour_singular: "1 ora fa",
        hour_plural: "{} ore fa",
        minute_singular: "1 minuto fa",
        minute_plural: "{} minuti fa",
        second_singular: "1 secondo fa",
        second_plural: "{} secondi fa",
    },
    rel_future: RelTime {
        day_singular: "tra 1 giorno",
        day_plural: "tra {} giorni",
        hour_singular: "tra 1 ora",
        hour_plural: "tra {} ore",
        minute_singular: "tra 1 minuto",
        minute_plural: "tra {} minuti",
        second_singular: "tra 1 secondo",
        second_plural: "tra {} secondi",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "º",
        two: "º",
        few: "º",
        other: "º",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " e ",
        pair: " e ",
    },
    now_str: "ora",
};

static NL_DATA: LocaleData = LocaleData {
    lang: "nl",
    region: "",
    plural_rule: PluralRule::simple_one(),
    decimal_sep: ",",
    thousands_sep: ".",
    dur_units: DurUnits {
        day_singular: "dag",
        day_plural: "dagen",
        hour_singular: "uur",
        hour_plural: "uren",
        minute_singular: "minuut",
        minute_plural: "minuten",
        second_singular: "seconde",
        second_plural: "seconden",
    },
    rel_past: RelTime {
        day_singular: "1 dag geleden",
        day_plural: "{} dagen geleden",
        hour_singular: "1 uur geleden",
        hour_plural: "{} uur geleden",
        minute_singular: "1 minuut geleden",
        minute_plural: "{} minuten geleden",
        second_singular: "1 seconde geleden",
        second_plural: "{} seconden geleden",
    },
    rel_future: RelTime {
        day_singular: "over 1 dag",
        day_plural: "over {} dagen",
        hour_singular: "over 1 uur",
        hour_plural: "over {} uur",
        minute_singular: "over 1 minuut",
        minute_plural: "over {} minuten",
        second_singular: "over 1 seconde",
        second_plural: "over {} seconden",
    },
    ordinal_suffixes: OrdinalSuffixes {
        one: "e",
        two: "e",
        few: "e",
        other: "e",
    },
    list_conj: ListPatterns {
        start: "",
        middle: ", ",
        end: " en ",
        pair: " en ",
    },
    now_str: "nu",
};

// ── Lookup table ─────────────────────────────────────────────────

static LOCALES: &[(&str, &LocaleData)] = &[
    ("en", &EN_DATA),
    ("pt", &PT_DATA),
    ("es", &ES_DATA),
    ("fr", &FR_DATA),
    ("de", &DE_DATA),
    ("ar", &AR_DATA),
    ("ru", &RU_DATA),
    ("ja", &JA_DATA),
    ("zh", &ZH_DATA),
    ("it", &IT_DATA),
    ("nl", &NL_DATA),
];

/// Look up locale data by language code (lowercase).
pub fn lookup_locale(lang: &str) -> Option<&'static LocaleData> {
    LOCALES
        .iter()
        .find(|(code, _)| *code == lang)
        .map(|(_, data)| *data)
}
