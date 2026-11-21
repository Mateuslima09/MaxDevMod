import type { CompanionWords } from '.'
import { words as spain } from './es_es'

/** Spanish (Latin America): Spain's Spanish, with Latin American usage where it differs (no "ha" perfect
 * for what just happened, "computadora", "mouse", "presionar", "videos"). */
export const words: CompanionWords = {
  ...spain,
  character: {
    ...spain.character,
    enableDetail:
      'Con el camión detenido y el freno de mano puesto, presiona {key} para bajar. Para volver a subir, acércate a la puerta del conductor y presiona {key}; mantén {key} 2 segundos para regresar desde cualquier lugar. Camina con W, A, S, D; Shift corre, Espacio salta, Ctrl se agacha.',
    speed: 'Sensibilidad del mouse',
    speedDetail: 'Qué tan rápido gira la vista mientras caminas. En el juego, - y = también la cambian.',
    walkKey: 'Tecla para bajar y subir',
    walking: 'caminar a pie',
    person: 'Quién camina',
    man: 'Hombre {n}',
    woman: 'Mujer {n}',
    names: {
      tattooed: 'Tatuado',
      madam: 'Madame',
      farmer: 'Granjero',
      gunslinger: 'Pistolero',
      stout: 'Gordito',
      builder: 'Ingeniera',
    },
    personNote: 'Se aplica la próxima vez que bajes.',
    restartNote: 'El juego está abierto: reinícialo para que se aplique el cambio de personaje.',
    hints: 'Barra de teclas en el juego',
    hintsDetail: 'Muestra abajo en la pantalla las teclas de lo que puedes hacer ahora: bajar, subir, el celular y la pantalla.',
    hintsKey: 'Tecla para mostrar y ocultar',
    hintsKeyDetail: 'Retroceso quita la tecla.',
    papers: 'Factura y enganche a mano',
    papersDetail: 'En los trabajos con tu camión: firmar la factura con el empleado y enganchar el remolque a pie (E). Desactivado, el juego lo hace como siempre.',
    refuel: 'Repostar a mano',
    refuelDetail: 'En el surtidor: bajar y repostar por el depósito del camión, con la manguera en la mano (E). Desactivado, Enter en la cabina reposta como siempre.',
    payAttendant: 'Pagar al empleado',
    payAttendantDetail: 'Después de repostar a pie, pagar al empleado delante del camión, con Nupay en el móvil (E). Desactivado, queda pagado.',
    noKey: 'Ninguna',
  },
  screen: {
    ...spain.screen,
    titles: { ...spain.screen.titles, stopped: 'Algo falló' },
    subs: {
      ...spain.screen.subs,
      stopped: 'El motor de MaxDevMod se detuvo. Tienes los detalles en Información.',
    },
  },
  controls: {
    ...spain.controls,
    sub: 'Mantén presionada una tecla para usar la pantalla del camión con el mouse. Al soltarla, vuelves a mirar por la cabina como siempre.',
    key: 'Mantén presionado para usar la pantalla',
    keyToggle: 'Presiona para usar la pantalla',
    toggle: 'Presionar una vez en lugar de mantener',
    toggleDetail: 'Presiona {key} para usar la pantalla y vuelve a presionarla para mirar por la cabina.',
    pressKey: 'Presiona una tecla…',
    keybindsNote:
      'Haz clic en un atajo y presiona su nueva tecla. Si mantienes {key} al presionarla, solo funciona mientras usas la pantalla. Esc cancela.',
  },
  accounts: {
    ...spain.accounts,
    sub: 'Inicia sesión para ver tu música y tus videos en la pantalla del camión. Inicias sesión en cada servicio en su propia ventana, y MaxDevMod nunca ve tu cuenta.',
  },
  map: {
    ...spain.map,
    titles: { ...spain.map.titles, failed: 'No se pudo preparar el mapa' },
  },
  tutorial: {
    ...spain.tutorial,
    hold: {
      title: 'Mantén presionado {key}',
      sub: 'En tu cabina, usas la pantalla mientras mantienes presionada esta tecla. Pruébalo ahora.',
      banner: { title: 'Mantén presionado {key}', text: 'Para usar la pantalla' },
    },
    open: {
      title: 'Haz clic en una app',
      sub: 'Sigue presionando: el mouse está ahora en la pantalla.',
      banner: { title: 'El mouse está en la pantalla', text: 'Haz clic en una app' },
    },
    letGo: {
      ...spain.tutorial.letGo,
      sub: 'Suelta y vuelves a mirar por la cabina con el mouse. Conduces con el teclado todo el tiempo.',
    },
    holdAgain: { title: 'Mantén presionado {key}', text: 'Para usar la pantalla' },
    tour: { ...spain.tutorial.tour, controls: 'Cambia la tecla que mantienes presionada y la velocidad del puntero.' },
  },
  uninstall: {
    ...spain.uninstall,
    text: 'MaxDevMod se quita de tu juego y de tus perfiles, y se elimina de esta computadora junto con tus inicios de sesión y sus registros.',
    steps: { ...spain.uninstall.steps, data: 'Eliminando MaxDevMod de esta computadora' },
    profilesFailed:
      'No se pudieron cambiar {n} de tus perfiles y todavía incluyen MaxDevMod. Quítalo allí en el Gestor de mods.',
  },
}
