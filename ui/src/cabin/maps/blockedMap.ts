/**
 * What the Maps app says instead of the map when the player plays on a map MaxDevMod doesn't allow (the host
 * tells: a map by HVT, Heavy Vehicles Turkey), in the game's language: every language the game has, by
 * its own codes (`locale/<code>` in its archives). HVT and MaxDevMod are names, the same in every language.
 * The game shows Silesian players Polish (pl_si), and so does this.
 */
export const HVT_MAP_MESSAGES: Record<string, string> = {
  bg_bg: 'Картите от HVT не са разрешени в MaxDevMod. Свържете се с MaxDevMod, за да научите повече.',
  ca_es: "Els mapes d'HVT no estan permesos a MaxDevMod. Contacta amb MaxDevMod per saber-ne més.",
  cs_cz: 'Mapy od HVT nejsou ve MaxDevMod povoleny. Pro více informací kontaktujte MaxDevMod.',
  da_dk: 'Kort fra HVT er ikke tilladt i MaxDevMod. Kontakt MaxDevMod for at få mere at vide.',
  de_de: 'Karten von HVT sind in MaxDevMod nicht erlaubt. Kontaktiere MaxDevMod, um mehr zu erfahren.',
  el_gr: 'Οι χάρτες της HVT δεν επιτρέπονται στο MaxDevMod. Επικοινωνήστε με το MaxDevMod για να μάθετε περισσότερα.',
  en_gb: 'Maps by HVT are not allowed in MaxDevMod. Contact MaxDevMod to learn more.',
  en_us: 'Maps by HVT are not allowed in MaxDevMod. Contact MaxDevMod to learn more.',
  es_es: 'Los mapas de HVT no están permitidos en MaxDevMod. Contacta con MaxDevMod para saber más.',
  es_la: 'Los mapas de HVT no están permitidos en MaxDevMod. Comunícate con MaxDevMod para saber más.',
  et_ee: 'HVT kaardid ei ole MaxDevMods lubatud. Lisateabe saamiseks võta ühendust MaxDevModga.',
  eu_es: 'HVTren mapak ez daude baimenduta MaxDevModn. Jarri harremanetan MaxDevModrekin gehiago jakiteko.',
  fi_fi: 'HVT:n kartat eivät ole sallittuja MaxDevModssa. Ota yhteyttä MaxDevModhin saadaksesi lisätietoja.',
  fr_ca: 'Les cartes de HVT ne sont pas permises dans MaxDevMod. Communiquez avec MaxDevMod pour en savoir plus.',
  fr_fr: 'Les cartes de HVT ne sont pas autorisées dans MaxDevMod. Contactez MaxDevMod pour en savoir plus.',
  gl_es: 'Os mapas de HVT non están permitidos en MaxDevMod. Contacta con MaxDevMod para saber máis.',
  hr_hr: 'Karte od HVT-a nisu dopuštene u MaxDevModu. Za više informacija obratite se MaxDevModu.',
  hu_hu: 'A HVT térképei nem engedélyezettek a MaxDevModben. További információért lépj kapcsolatba a MaxDevModjel.',
  it_it: 'Le mappe di HVT non sono consentite in MaxDevMod. Contatta MaxDevMod per saperne di più.',
  ja_jp: 'HVTのマップはMaxDevModでは使用できません。詳しくはMaxDevModにお問い合わせください。',
  ka_ge: 'HVT-ის რუკები MaxDevMod-ში დაშვებული არ არის. მეტის გასაგებად დაუკავშირდით MaxDevMod-ს.',
  ko_kr: 'HVT의 지도는 MaxDevMod에서 허용되지 않습니다. 자세한 내용은 MaxDevMod에 문의하세요.',
  lt_lt: 'HVT žemėlapiai MaxDevMod neleidžiami. Norėdami sužinoti daugiau, susisiekite su MaxDevMod.',
  lv_lv: 'HVT kartes MaxDevMod nav atļautas. Lai uzzinātu vairāk, sazinieties ar MaxDevMod.',
  mk_mk: 'Мапите од HVT не се дозволени во MaxDevMod. Контактирајте го MaxDevMod за да дознаете повеќе.',
  nl_nl: 'Kaarten van HVT zijn niet toegestaan in MaxDevMod. Neem contact op met MaxDevMod voor meer informatie.',
  no_no: 'Kart fra HVT er ikke tillatt i MaxDevMod. Kontakt MaxDevMod for å finne ut mer.',
  pl_pl: 'Mapy od HVT nie są dozwolone w MaxDevMod. Skontaktuj się z MaxDevMod, aby dowiedzieć się więcej.',
  pl_si: 'Mapy od HVT nie są dozwolone w MaxDevMod. Skontaktuj się z MaxDevMod, aby dowiedzieć się więcej.',
  pt_br: 'Mapas da HVT não são permitidos no MaxDevMod. Entre em contato com o MaxDevMod para saber mais.',
  pt_pt: 'Os mapas da HVT não são permitidos no MaxDevMod. Contacte o MaxDevMod para saber mais.',
  ro_ro: 'Hărțile HVT nu sunt permise în MaxDevMod. Contactează MaxDevMod pentru a afla mai multe.',
  ru_ru: 'Карты от HVT не разрешены в MaxDevMod. Свяжитесь с MaxDevMod, чтобы узнать больше.',
  sk_sk: 'Mapy od HVT nie sú vo MaxDevMod povolené. Pre viac informácií kontaktujte MaxDevMod.',
  sl_sl: 'Zemljevidi HVT v MaxDevModu niso dovoljeni. Za več informacij se obrnite na MaxDevMod.',
  sr_sp: 'Mape od HVT-a nisu dozvoljene u MaxDevMod-u. Kontaktirajte MaxDevMod da saznate više.',
  sr_sr: 'Мапе од HVT-а нису дозвољене у MaxDevMod-у. Контактирајте MaxDevMod да сазнате више.',
  sv_se: 'Kartor från HVT är inte tillåtna i MaxDevMod. Kontakta MaxDevMod för att få veta mer.',
  tr_tr: "HVT haritalarına MaxDevMod'de izin verilmiyor. Daha fazla bilgi için MaxDevMod ile iletişime geçin.",
  uk_uk: "Карти від HVT не дозволені у MaxDevMod. Зв'яжіться з MaxDevMod, щоб дізнатися більше.",
  vi_vn: 'Bản đồ của HVT không được phép sử dụng trong MaxDevMod. Liên hệ MaxDevMod để tìm hiểu thêm.',
  zh_cn: 'MaxDevMod 不允许使用 HVT 的地图。如需了解详情，请联系 MaxDevMod。',
  zh_tw: 'MaxDevMod 不允許使用 HVT 的地圖。如需瞭解詳情，請聯絡 MaxDevMod。',
}

/** The message in the game's `language`, or English for one not listed (a mod's). */
export function hvtMapMessage(language: string | null): string {
  return HVT_MAP_MESSAGES[language ?? ''] ?? HVT_MAP_MESSAGES.en_us
}
