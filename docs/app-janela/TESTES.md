# Janela do app (Contas e Mapa): testes para o dono

## Erro STATUS_BREAKPOINT ao abrir Contas ou Mapa (2026-11-20)

**O que acontecia:** ao clicar em **Contas** ou **Mapa** na janela do MaxDevMod, aparecia a página de erro
"Esta página está com problemas" (código STATUS_BREAKPOINT).

**Por quê:** hoje às 10:38 o Windows atualizou sozinho o WebView2 (o "navegador" que desenha a janela do app) para
a versão 158. Essa versão tem um defeito: quando uma página dentro de um quadro (iframe) faz a animação de "abrir
app" (o ícone crescendo até virar a janela do app, a View Transition), o WebView2 trava a página inteira. A tela de
caminhão que aparece à direita na janela do app é justamente a tela da cabine dentro de um quadro. Contas abre o
app do YouTube nela, Mapa abre o app Mapas, e era nessa animação que tudo caía. (Controles também abre Música e
Mapas na demonstração, então também podia cair.) Nada no código do MaxDevMod mudou nessa parte desde a 2.1.4. Foi o
WebView2 novo.

Reproduzido no app de verdade: abrir qualquer app na tela de demonstração (até Música) derrubava a página. Uma
animação mínima dentro de um quadro também derrubava. A mesma animação fora de quadro funcionava. Por isso a tela
da cabine no jogo, que é uma página própria e não fica dentro de quadro, não é afetada.

**Correção:** dentro de um quadro (só a tela de demonstração da janela do app), a tela da cabine troca de app sem
essa animação. No jogo, a animação de abrir e fechar app continua igual.

Instalado em 2026-11-20 (app e plugin juntos, ponte 52). Para voltar: `westplay.exe.antes-contas-mapa`,
`westplay.dll.antes-contas-mapa` e a pasta `ui.antes-contas-mapa` em `%LOCALAPPDATA%\MaxDevMod`.

- [ ] Abra o MaxDevMod e clique em **Contas**: a página abre sem erro, e a tela à direita mostra o YouTube?
- [ ] Clique em **Mapa**: abre sem erro, e a tela à direita mostra o app Mapas?
- [ ] Clique em **Controles** e espere uns 10 s: a tela à direita abre e fecha Música e Mapas sem erro?
- [ ] Passe por todas as outras opções (Tela do caminhão, Personagem, Personalizar, Configuração, Sobre): nenhuma dá
      erro?
- [ ] No jogo, na tela da cabine, abra e feche um app: a animação do ícone crescendo continua lá?
