# GPS melhor em mods de mapa pagos (tarefa futura)

Pedido do dono (2026-11-12): em mods de mapa pagos (protegidos), o Maps só tem uma noção da rota, sem os detalhes do mapa. Ver se dá pra ler o que o próprio jogo lê, ou pelo menos ter mais detalhe no mapa.

**Etapa 0 feita (2026-11-12); nada de código ainda.** Este arquivo guarda o plano.

## Como é hoje (lido no código, 2026-11-12)

- O mapa do Maps é montado dos arquivos do jogo e dos mods de mapa do perfil (`crates/setup/src/map.rs`, `crates/mapdata`). Um mod cujos arquivos não dá pra ler (arquivo danificado ou protegido) é deixado de fora (`LeftOut`, `left-out.json`) e o mapa é montado com o resto; o companion mostra o nome dele (`ui/src/companion/room/parts.tsx`, `leftOut`).
- O que não se lê hoje (`crates/mapdata/src/archive.rs`): entradas criptografadas, ZIP64, HashFS versão 2 com salt, versões além de 1 e 2, métodos de hash desconhecidos.
- A rota vem da memória do jogo (`crates/host/src/route.rs`): os nós do navegador com posição e rotação. Isso funciona mesmo no mod protegido, porque é o jogo que já leu o mapa. Mas o desenho entre os nós (curvas, faixas dos cruzamentos: `westplay_mapdata::junctions`) e as estradas em volta vêm dos arquivos do mod, que não foram lidos. Resultado: a rota vira uma linha entre nós, sem estrada, cruzamento nem cidade em volta.

## Etapa 0 — descobrir (antes de qualquer código)

- Pegar 2 ou 3 mods pagos comuns e anotar por que cada um foi deixado de fora (a mensagem do `LeftOut`): criptografia, salt, versão, manifesto embaralhado… Pode ser que alguns sejam só um formato que ainda não lemos (ex.: HashFS v2 com salt) e não proteção de verdade. Esses são o caminho fácil.
- Ver se o mod fica de fora inteiro ou se dá pra ler parte (ex.: o `.sii` de definições sim e os setores não). Hoje um erro tira o mod todo; ler o que der já melhora.

### Resultado da etapa 0 (2026-11-12, neste PC)

Mods de mapa encontrados:

| Mod | Onde | Como é o arquivo |
|---|---|---|
| ME Base (4,0 GB), ME Paraná, ME Santa Catarina, ME São Paulo (1,4 GB cada) — Mapa Eldorado | `Documents\Euro Truck Simulator 2\mod` | Protegido pelo **ModFy** (ver abaixo) |
| Ativador de Cargas Mapa Eldorado PRO… (1,4 MB) | mesma pasta | ZIP normal, só definições de carga |
| MAPA_RBR_PARTE_3/5/6, MAPA_RBR_SEM_EUROPA, Norman_Mods_RBR_MP_Edition | `OneDrive\Documentos\Euro Truck Simulator 2\mod` | Não olhados: estão só na nuvem do OneDrive (não baixados, ~5,5 GB), e o perfil não usa |

O perfil do ETS2 ("Eu PRO") joga `map_path: /map/eldorado.mbd` com os 4 ME ativos. No ATS não há mod de mapa em nenhum perfil.

**Como o Mapa Eldorado é protegido (ModFy):**

- Cada `.scs` é um ZIP "casca": o diretório do ZIP (lido pelo fim do arquivo) só tem 5 arquivos pequenos e abertos: `manifest.sii` (`author: "Equipe Eldorado / ModFy"`, `category[]: "map"` no ME Base), `a.jpg` (ícone), `b.txt`/`b.pt_br.txt` ("Abra seu aplicativo ModFy para usar esse mod.") e `iz.txt` (um link de imagem do ModFy). Comentário do ZIP: "Open your ModFy to use this Mod.", e depois dele um rodapé próprio de 24 bytes terminando em `RL.IZ`.
- O mapa de verdade (quase todo o arquivo) é um bloco à parte no começo, com cabeçalho `RL\0RA` e o resto embaralhado/criptografado. Não é HashFS (`SCS#`), nem ZIP, nem ZIP64, nem salt: **é proteção de verdade, não um formato que falta ler.**
- O ModFy é um app instalado (`C:\Program Files\ModFy`, Electron) que **põe o `ModFy.dll` (91 MB na memória) dentro do jogo**: confirmado nos módulos carregados do `eurotrucks2.exe` com o jogo aberto. É ele que abre o bloco e entrega o mapa ao jogo, na memória. Nada é salvo aberto no disco (`AppData\Roaming\ModFy\Mod Cache` vazio).
- Pelos cuidados deste plano, não dá pra seguir por aqui: abrir o bloco seria quebrar a proteção. **A etapa 1 não ajuda no Mapa Eldorado.**

**O que o WestPlay faz hoje com ele (lido no código; o mapa ainda não foi tentado porque o jogo estava aberto):**

- Os ME não viram `LeftOut`: a casca abre sem erro, só não tem setores nem definições, então `affects_map` dá falso e eles nem contam como mods de mapa (`profiles.rs`, `map_mods`).
- O mapa do perfil é `eldorado`, que não existe nos arquivos que o WestPlay lê. Ao montar (com o jogo fechado), deve falhar inteiro com `the map eldorado has no roads` (`ExportError::Empty`, `export.rs`), e não com "mod deixado de fora". O companion mostra falha do mapa, não o nome do mod.
- Por isso o Maps fica só com a rota: ela vem da memória do jogo (`route.rs`) e funciona com o ModFy ligado (o log mostra a rota lida às 19:39:58 com o mapa Eldorado rodando). O `westplay.dll` e o `ModFy.dll` convivem no jogo sem problema.

**Conclusões pras próximas etapas:**

- O mapa Eldorado inteiro só existe aberto **na memória do jogo**, então a etapa 2 (ler o que o jogo carregou) e a 2b (GPS do jogo) são o caminho, não a 1. A leitura de fora, só leitura, já funciona hoje ao lado do ModFy.
- Um mapa próprio (`eldorado.mbd`) é o caso mais difícil: não há nem o mapa base embaixo. Um "mapa que se descobre dirigindo" (etapa 2) começaria vazio; a 2b-A (copiar a imagem do GPS) dá algo útil desde o primeiro minuto.
- Coisa pequena que já vale fazer (etapa 3): reconhecer um mod do ModFy (manifesto com `ModFy` no autor, ou o comentário do ZIP) e mostrar no companion "mapa protegido pelo ModFy: detalhes do mapa indisponíveis" no lugar de "o mapa eldorado não tem estradas".
- Ainda a conferir: se o RBR também é ModFy ou outra coisa (precisa baixar do OneDrive); como fica um mod pago que só **estende** o mapa base (tipo ProMods), em que o resto do mapa ainda sai.

## Etapa 1 — ler mais do que já está nos arquivos (fácil)

- Ler o que for só formato não suportado (salt na v2, ZIP64 etc.), se a etapa 0 mostrar que é isso.
- Deixar de tirar o mod inteiro por causa de uma entrada ruim: pular só a entrada, montar o resto do mod.

## Etapa 2 — ler o que o jogo já carregou (a ideia principal)

O jogo precisa abrir o mod pra rodar, então o mapa dele está na memória do jogo, já aberto. O host já lê a memória do jogo só para leitura (`route.rs`, `ReadProcessMemory`), do mesmo jeito daria pra ler mais:

- A tabela de nós do jogo (a mesma que o `goto` do console e o `navigation.rs` do plugin usam): posição de todos os nós carregados, inclusive os do mod. Com os nós e as ligações entre eles já dá pra desenhar as estradas por cima do mapa base.
- Os itens do mapa carregados (estradas, prefabs/cruzamentos, cidades, empresas), pelo menos dos setores perto do caminhão. Ir guardando conforme o jogador passa (um "mapa que se descobre dirigindo"), num arquivo do WestPlay, por perfil.
- Os pontos do próprio GPS do jogo entre os nós (o caminho que ele desenha no mapa dele), se ficarem na memória, pra rota sair com curva mesmo sem os cruzamentos.

Como no resto: achar tudo pelo formato no código do jogo, nunca por endereço fixo, conferir, e não fazer nada se não bater (trava só a função, não o app).

## Etapa 2b — usar o GPS original do jogo como base (pedido do dono, 2026-11-12)

O GPS que o jogo mostra na tela do caminhão já tem tudo desenhado, com o mod pago incluído, porque é o jogo que desenha. Duas formas de aproveitar:

### A. Copiar a imagem do GPS do jogo (a mais rápida de testar)

- O plugin já fica no meio do desenho dessa tela: o mod do WestPlay põe a nossa imagem como a camada de cima do GPS do jogo, e o plugin enche essa imagem quando o jogo vai desenhá-la (`crates/plugin/src/screens.rs`, `on_set_render_targets` em `d3d.rs`). Nesse momento o jogo já desenhou o mapa dele por baixo, no mesmo alvo. Dá pra copiar esse alvo antes da nossa camada e mandar a imagem pro app (como o app já manda as dele pro jogo, pelo bridge, só que no sentido contrário).
- Bom: é exatamente o que o jogo mostra, mod pago incluso, sem ler arquivo nenhum.
- Ruim: é uma foto. Zoom, ângulo, estilo e resolução são os do jogo (a tela do GPS é pequena); não dá pra deixar no visual do WestPlay nem mexer no mapa à vontade. Serve como um modo "GPS original" ou só onde o mod ficou de fora.
- A conferir: se o jogo desenha o GPS mesmo com a câmera fora da cabine, ou com a tela do caminhão desligada.

**Feito e depois retirado (2026-11-12, decisão do dono):** o Mapas voltou a ser como era. Quem quiser o GPS original do jogo já pode desligar o WestPlay ("Mostrar o WestPlay" no companion), então a foto no Mapas não fazia falta. O que tinha sido feito, para o caso de voltar a ela:

- Plugin (`crates/plugin/src/game_gps.rs`): quando o jogo vai desenhar a nossa camada numa tela de caminhão (não o tablet), copia a parte do alvo em que o jogo desenha (o viewport) para uma textura nossa de leitura, no máximo 1 a cada 15 quadros e só enquanto o app pede. A leitura é 2 quadros depois, sem esperar a placa de vídeo. O log do plugin diz uma vez o tamanho copiado (`game gps: copying …`).
- Memória compartilhada própria (`crates/shared/src/game_gps.rs`, `Local\WestPlay.GameGps.v1`), separada do bridge: a versão do bridge não mudou.
- App (`crates/host/src/game_gps.rs`): numa thread própria, pede as fotos enquanto a página quer, transforma em JPEG e manda pra página (`gameGps`). O log do app diz `game gps: the first picture …`.
- Página (`ui/src/cabin/maps/gameGps.ts`, `MapView.tsx`): no mapa sem mapa do jogador (o caso do Eldorado), pede as fotos e mostra a do jogo por cima, com o selo "GPS do jogo", sem os botões de zoom. Se as fotos param de chegar por 2,5 s (câmera fora da cabine, tela desligada), volta o mapa em branco.
- Conferido no navegador com uma foto simulada: aparece, e some quando param de chegar. **Não conferido no jogo**: se o mapa do jogo já está mesmo desenhado no alvo quando a nossa camada entra, e se o jogo desenha o GPS por baixo da nossa tela.

### B. Ler o que o jogo usa pra desenhar o GPS (mais detalhe, mais trabalho)

- Pra desenhar o GPS o jogo monta, na memória, as estradas em volta e a linha da rota já com as curvas (os pontos que vão pra tela). Ler esses pontos (só leitura, como o `route.rs` já faz com a rota) e desenhar com o nosso visual, com zoom e rotação livres.
- Começar pela linha da rota (o que mais falta hoje), depois as estradas em volta.
- Achar pelo formato, nunca por endereço fixo; muda a cada versão do jogo, então tem que conferir e cair fora limpo se não bater.

**Juntando:** onde temos o mapa (mods abertos), segue o nosso; onde o mod ficou de fora, usar B (ou A enquanto B não existir).

## Etapa 3 — mais detalhe sem ler o mod (se as de cima não derem)

- Desenhar a rota com curva suave entre os nós (usando a rotação de cada nó, que já é lida) em vez de linha reta.
- Mostrar no mapa que aquela região é de um mod que não deu pra ler ("detalhes indisponíveis"), em vez de parecer um erro.

## Cuidados

- Mods pagos são protegidos de propósito pelos autores. O que for lido serve só pra desenhar no aparelho do próprio jogador: nunca exportar, juntar num arquivo que dê pra passar adiante, nem quebrar a criptografia dos arquivos. Ler o que o jogo já abriu na memória, para uso local, é o caminho mais limpo. Vale conferir os termos dos mods mais usados antes.
- Ler a memória do jogo continua só leitura e de fora do jogo (sem nada injetado além do plugin que já existe).

## Pronto quando

- Num perfil com um mod pago, o Maps mostra as estradas do mod (ao menos as que o jogador já passou) e a rota com curvas, não só uma linha entre pontos.
