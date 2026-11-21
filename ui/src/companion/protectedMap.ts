/**
 * What the companion says of a profile whose map comes from a protected map mod (the host's `protectedMap`: no
 * roads, its mod in a format of its own only the game opens; owner 2026-11-02, Mapa Eldorado), rather than the
 * build's error: not a fault, and the rest of MaxDevMod works. `{map}` is the map's name. In the companion's
 * language (its code, `pt_br`), by its language where there's no word for its country, else English.
 */
const MESSAGES: Record<string, string> = {
  en_us:
    'The map “{map}” comes from a protected mod that only the game can open. MaxDevMod can’t read protected mods, so this profile has no map in MaxDevMod; everything else works as usual.',
  pt_br:
    'O mapa “{map}” vem de um mod protegido, que só o jogo consegue abrir. O MaxDevMod não lê mods protegidos, então este perfil fica sem mapa no MaxDevMod; o resto funciona normalmente.',
  pt_pt:
    'O mapa “{map}” vem de um mod protegido, que só o jogo consegue abrir. O MaxDevMod não lê mods protegidos, por isso este perfil fica sem mapa no MaxDevMod; o resto funciona normalmente.',
  es_es:
    'El mapa «{map}» viene de un mod protegido que solo el juego puede abrir. MaxDevMod no lee mods protegidos, así que este perfil no tiene mapa en MaxDevMod; todo lo demás funciona con normalidad.',
  tr_tr:
    '“{map}” haritası yalnızca oyunun açabildiği korumalı bir moddan geliyor. MaxDevMod korumalı modları okuyamaz, bu yüzden bu profilin MaxDevMod’de haritası yok; geri kalan her şey normal çalışır.',
  de_de:
    'Die Karte „{map}“ stammt aus einer geschützten Mod, die nur das Spiel öffnen kann. MaxDevMod kann geschützte Mods nicht lesen, daher hat dieses Profil in MaxDevMod keine Karte; alles andere funktioniert wie gewohnt.',
  fr_fr:
    'La carte « {map} » vient d’un mod protégé que seul le jeu peut ouvrir. MaxDevMod ne lit pas les mods protégés : ce profil n’a donc pas de carte dans MaxDevMod ; tout le reste fonctionne normalement.',
  it_it:
    'La mappa “{map}” proviene da una mod protetta che solo il gioco può aprire. MaxDevMod non legge le mod protette, quindi questo profilo non ha una mappa in MaxDevMod; tutto il resto funziona normalmente.',
}

export function protectedMapMessage(code: string, map: string): string {
  const language = code.split('_')[0]
  const message =
    MESSAGES[code] ?? Object.entries(MESSAGES).find(([key]) => key.startsWith(`${language}_`))?.[1] ?? MESSAGES.en_us
  return message.replace('{map}', map)
}
