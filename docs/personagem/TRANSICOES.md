# Transições de animação e pés deslizando: diagnóstico e plano (2026-11-06)

Resposta à etapa 1 do pedido do dono (`prompt_transicoes_animacoes_ets2_ats.pdf`). Tudo aqui foi conferido no
código e medido no Blender; o que não foi confirmado está marcado como hipótese.

## 1. Como funciona hoje

**Carregar e escolher a animação.** Cada animação é um `mover_model` no mod
(`def/world/mover.westplay_player.sii`, gerado por `crates/setup/src/own_character.rs`): um modelo (`.pmd/.pmg`) e
**uma** animação (`anim/<nome>.pma`). O plugin cria uma "pessoa do jogo" para cada uma (9 por personagem), todas
posicionadas a cada quadro. **Só uma é desenhada** (`DRAWN`, `crates/plugin/src/character/person.rs`).

**Movimento.** Quem anda é a cápsula de física da câmera de andar do jogo. O plugin só lê a velocidade dela e
limita a velocidade (`limit_speed`): **1,6 m/s andando, 3,2 correndo, 0,9 agachado**, iguais para todos
(`crates/plugin/src/character.rs`). O corpo vira para onde a cápsula foi, a 10 rad/s (`TURN_SPEED`).

**Troca de animação.** É **instantânea**: a pessoa desenhada passa a ser outra (`place`, o `role`). A de antes
fica 2 quadros a mais para não piscar (`DRAWN_BEFORE`). Cada pessoa tem o **próprio relógio**: o do parado anda
sempre, o do andar só anda andando, o da corrida anda sempre.

**Esqueleto e arquivos.** Um esqueleto por personagem (`xx_player.pis`), o mesmo para o corpo sem cabeça e o com
celular (a marca de 8 bytes no `.pmg`/`.pma` igual). Fonte: `.pim/.pis/.pia/.pit` (SCS Blender Tools); jogo:
`.pmd/.pmg/.pma/.dds` (o conversor da SCS).

**O que o motor permite, confirmado:**
- tocar uma animação por pessoa, em laço, e **andar o relógio dela quanto quisermos a cada quadro** (é assim que
  o pulo e o agachar começam no quadro certo e esperam);
- portanto, **a velocidade de cada animação pode ser ajustada**, quadro a quadro (o relógio anda `segundos × fator`).

**O que não foi encontrado (não existe no caminho que usamos):** misturar duas animações na mesma pessoa
(*blend*), interpolar entre elas, ou deixar uma pessoa meio transparente para fazer um *crossfade*. O jogo tem os
pedestres dele, mas não achamos como ele mistura animações, nem se mistura. **Toda transição precisa ser uma
animação pronta, feita no Blender.**

## 2. A causa da troca brusca para o parado

Soltando o W, o andar some e aparece o parado **na pose em que o relógio dele estiver**, que não tem relação com
a pose do andar naquele momento: os braços pulam do meio do passo para o lado do corpo. Na mulher e no tatuado,
o "parar" (o fim do *Female Stop Walking*) melhora um pouco, mas também começa sempre do mesmo quadro (31),
qualquer que seja o passo em que você soltou. Por isso ainda dá um tranco no começo, e outro pequeno no fim (o
quadril sobe 2 cm até o parado). Da corrida para o andar acontece o mesmo: os dois relógios não estão sincronizados.

## 3. Pés deslizando: medido

As animações são todas *in place* (o quadril não sai do lugar: o conferidor mede). Então, para os pés não
escorregarem, a cápsula tem de andar na **mesma velocidade em que o pé apoiado vai para trás** na animação.
Medido no Blender (a velocidade do pé no chão, mediana dos quadros de apoio):

| Personagem | Animação | Pé no chão (m/s) | Cápsula hoje (m/s) | Diferença |
|---|---|---|---|---|
| Mulher | andar | 1,49 | 1,6 | o corpo 7% mais rápido que os pés |
| Mulher | correr | 3,27 | 3,2 | certo |
| Mulher | andar com celular | **1,10** | 1,6 | **o corpo 45% mais rápido: escorrega** |
| Tatuado | andar (o do primeiro homem) | 1,52 | 1,6 | 5% |
| Tatuado | correr (o do primeiro homem) | ~4,2 | 3,2 | **os pés 30% mais rápidos: "patina no gelo"** |
| Tatuado | andar com celular | 1,08 | 1,6 | **45%** |
| (primeiro homem) | andar / correr | 1,73 / 4,49 | 1,6 / 3,2 | 8% / 40% |

O parado é estável (o pé fica parado: 0 m/s). Agachado não há animação de andar agachado: andando a 0,9 m/s, a
pose fica parada e desliza (é o que existe hoje, não é defeito de sincronia). Ao parar ou arrancar a cápsula
acelera e freia, mas a animação anda sempre na velocidade cheia: nesse meio segundo os pés escorregam.

**Curvas:** o corpo vira para onde a cápsula vai, rápido (10 rad/s), sem animação de virar. Numa curva fechada
parada (só virando) os pés giram no lugar. É uma hipótese, não medida no jogo, que isso pareça "patinar de lado".

## 4. A solução, dentro do que o motor deixa

**Pés (etapa 3), no plugin, sem mudar animação nenhuma:**
1. Cada perfil ganha a velocidade natural de cada passo (a tabela acima), e o limite da cápsula passa a ser
   ela. Para a mulher, o andar com celular passa para 1,1 m/s. Para o tatuado, a corrida passa de 3,2 para
   perto da natural, até um teto que o dono aprovar.
2. A animação de andar ou correr passa a tocar na **velocidade real da cápsula** (fator = velocidade ÷ natural,
   entre 0,6 e 1,3). Assim, arrancando e freando, os passos acompanham e os pés não escorregam.

**Transições (etapas 2 e 4), animações novas geradas no Blender a partir das que já existem:**
- **Andar → parado:** 4 animações curtas (~0,5 s), cada uma começando num quarto do ciclo do andar e terminando
  exatamente no primeiro quadro do parado. Elas são feitas misturando, quadro a quadro, o andar (continuando do
  ponto onde parou) com o parado, com o pé no chão. Ao soltar o W, o plugin espera o próximo quarto do ciclo
  (no máximo 0,26 s, em média 0,13 s), toca a transição dele e começa o parado **do quadro 1**. Assim nenhuma
  troca tem pose diferente. Se você andar de novo no meio, o andar volta no ponto do ciclo em que a transição
  começou mais o tempo que passou (sem recomeçar o passo).
- **Correr → parado:** o mesmo, com a corrida.
- **Andar ↔ correr:** sem animação nova: os dois relógios passam a andar juntos, na mesma fração do ciclo (os
  dois têm 2 passos por ciclo). A troca cai no mesmo pé.
- **Parado → andar:** o andar começa do quadro em que o pé de trás vai sair do chão, em vez de onde o relógio
  estiver.
- O pulo (subir, cair, pousar, voltar a andar) já é assim, com tempos medidos.

Custo: 8 animações a mais por personagem (4 do andar, 4 da corrida), mais as 8 do corpo sem cabeça. São mais 16
pessoas posicionadas a cada quadro, mas só uma é desenhada. Isso **precisa ser medido** no notebook (RTX 2050) antes de
seguir para a corrida: o protótipo começa só com o andar → parado.

## 5. Arquivos que vão mudar

- `tools/personagem/transicao_parar.py` (novo): gera a transição (`settle`, 8 pedaços de 0,3 s a partir da pose do andar congelada) a partir de `andar` e `parado`. A 1ª versão (4 transições em que o andar continuava) deixou o personagem dando passos parado.
- `crates/setup/src/own_character.rs`: os nomes novos (`w2i0`…`w2i3`, depois `r2i0`…`r2i3`).
- `crates/plugin/src/character/person.rs`: o perfil com as velocidades naturais e as transições; a escolha do
  quarto do ciclo; o parado do quadro 1; a velocidade da animação pela da cápsula.
- `crates/plugin/src/character.rs`: o limite de velocidade por perfil e por passo.
- Cópias de segurança: `.antes-transicao` nos arquivos instalados, e a cena `_animada` copiada antes.

## 6. Como testar (cada etapa)

- Etapa 2: andando, soltar o W em vários momentos do passo, olhando os braços: a parada é contínua, sem tranco?
  Apertar W de novo no meio da parada. Subir e descer do caminhão.
- Etapa 3: andar e correr retos olhando os pés no chão (3ª pessoa, de lado); arrancar e frear; andar com o
  celular; fazer curvas.
- O log do plugin mostra cada troca (`shown ...`). Os números ficam no `TESTES.md`.

---

# 7. Revisão técnica do plano (2026-11-06, pedida pelo dono)

Pedido: questionar o plano acima antes de autorizar o resto. Nesta revisão **nada foi implementado**. Tudo o que
segue foi conferido no código (`crates/plugin/src/character.rs`, `character/person.rs`) ou medido no Blender nas
cenas da mulher (`mulher_animada.blend`) e do tatuado (`tatuado_animado.blend`).

**Antes de tudo, uma correção de rota.** A etapa 2 já foi implementada duas vezes antes desta revisão:
- **1ª versão (19:15):** as 4 transições em que o andar continuava, esperando o próximo quarto do ciclo. No teste
  do dono ficou **pior**: "faz duas, três animações já parado, como se escorregasse". Até ~0,9 s de passos com o
  corpo parado (0,15 s de confirmação + até 0,26 s de espera + 0,5 s de transição).
- **2ª versão (19:25, instalada, ainda não testada):** uma animação `settle` com 8 pedaços de 0,3 s, cada um da
  pose do andar **congelada** até o primeiro quadro do parado, tocada na hora.

Por isso as seções 4 e 5 acima descrevem a 1ª versão, já descartada. Esta revisão avalia o plano original, a 2ª
versão e o que falta.

## 7.1 Sincronizar a animação com a velocidade real da cápsula

**Como a velocidade é lida hoje:** pela diferença de posição da cápsula entre dois quadros (`went`,
`character.rs:1175`), sem filtro. Nos quadros em que a física do jogo não roda, o deslocamento é zero. É por isso
que já existe a confirmação de 0,08 s para "andando" e 0,15 s para "parado" (`MOVING_AFTER`, `STILL_AFTER`).

**Usar essa velocidade como relógio da animação (fator = velocidade ÷ velocidade natural) é o caminho certo para
os pés não escorregarem**, porque o motor deixa andar o relógio de cada pessoa quanto quisermos. Mas, do jeito que
estava no plano, tem problemas:

1. **A velocidade crua oscila.** Um quadro sem física dá fator 0, e o próximo dá o dobro: as pernas "tremem".
   É preciso **filtrar** a velocidade (média dos últimos ~0,1–0,15 s) antes de usar como fator.
2. **Desacelerando, as pernas quase congelam.** Com a cápsula a 0,3 m/s e a velocidade natural de 1,5 m/s, o
   fator dá 0,2: uma pose quase parada, como a "estátua" de antes. Precisa de um **piso** (≈0,5). Abaixo dele, o
   personagem já deve estar entrando no parado (a transição), não andando devagar.
3. **Acelerando, o contrário:** nos primeiros décimos de segundo o fator é baixo e o primeiro passo sai "em câmera
   lenta". Isso é aceitável e até natural, mas **precisa ser medido**: não sabemos quanto tempo a câmera de andar
   do jogo leva para chegar a 1,6 m/s. Antes de mexer, registrar no log a velocidade quadro a quadro ao apertar e
   ao soltar o W.
4. **Um teto também é preciso (≈1,3).** Com o celular, a mulher anda a 1,6 m/s, mas a animação foi feita para
   1,1: o fator daria 1,45, passos apressados e artificiais. **Para essa animação o certo é limitar a cápsula a
   1,1 m/s com o celular aberto**, e não acelerar as pernas.
5. **Cada animação de cada personagem tem a sua velocidade natural**, e as medidas atuais não são todas
   confiáveis:
   - a mulher andando (1,49), correndo (3,27) e andando com o celular (1,10): boas, com 11 a 99 amostras;
   - a corrida do tatuado: **~4,2 m/s com só 2 amostras**. Não serve: é preciso medir de outro jeito, pelo
     comprimento do passo (a distância entre os pés em cada pisada × passos por segundo) e conferir com a
     velocidade do pé apoiado.

   A corrida do tatuado (a do primeiro homem) é longa: ~4,2 m/s, contra 3,2 m/s da cápsula. Ou se aceita fator
   ~0,75 (as pernas visivelmente lentas), ou se sobe a corrida dele para ~3,8 m/s (fator ~0,9), ou se troca a
   animação por uma corrida mais curta. **É uma decisão do dono**, não técnica.

**Mudança de direção:** o corpo vira 10 rad/s para onde a cápsula vai (`TURN_SPEED`), e a velocidade usada é o
módulo, então virar não muda o fator. Em curva aberta não há problema. Virando muito (para trás com o S, ou um
"L" fechado), o corpo gira ~180° em ~0,3 s com os pés no ritmo da caminhada: um giro "patinado" curto. Não há
animação de virar. Isso só se resolve com animações de giro do Mixamo (*Left/Right Turn*), e fica para depois.

**Os passos (som) acompanham sozinhos**, porque usam o mesmo relógio (`Body::steps`).

## 7.2 Andar → parado (o que foi testado e o que deve ficar)

| Pergunta | Conclusão |
|---|---|
| 4 transições, cada uma começando num quarto do ciclo, bastam? | **Não.** Esperar o próximo quarto dá até 0,26 s de passos com o corpo já parado, e a 1ª versão provou que isso aparece. Com **8 pedaços tocados na hora** (2ª versão), a pose do andar e o começo do pedaço ficam a no máximo 1/16 do ciclo (~0,06 s de movimento). Não há espera. |
| A transição de ~0,5 s é adequada? | **Não, se o andar continuar dentro dela** (foi o erro da 1ª versão). Com a pose congelada, **0,3 s** fecha os pés sem passo a mais. Pode até ser curto demais para um passo natural: só o teste dirá. É um número fácil de mudar. |
| Como garantir que o fim é igual ao 1º quadro do parado? | No Blender, o último quadro de cada pedaço é **o próprio quadro 1 do parado** (a mistura com peso 1). O "pé no chão" depois desloca o quadril em milímetros nesse quadro, porque o parado já tem o pé no chão. No plugin, o relógio do parado é levado ao 0 no mesmo quadro em que a transição acaba (`stand_by`). Garantido por construção, **mas falta conferir no jogo** (a 2ª versão ainda não foi testada). |
| Voltar a andar no meio da transição | O andar volta da fase em que estava ao soltar o W. O corpo, porém, já foi um pouco em direção ao parado (até 0,3 s de mistura), então pode haver um pequeno salto de volta. É menor que o salto original e **aceitável no 1º protótipo**. A solução completa seria tocar o pedaço "de trás para frente", mas o relógio do motor só anda para frente (`update(instance, seconds)` com `seconds ≥ 0`). |

**O que a 2ª versão ainda não resolve:** os 0,15 s de confirmação de parada (`STILL_AFTER`). Neles o andar continua
na velocidade cheia com a cápsula freando, e os pés escorregam um pouco. A sincronia da 7.1 resolve isso junto: as
pernas desaceleram com a cápsula.

**Os pés durante a transição:** misturar rotações move os dois pés ao mesmo tempo. O pé que estava apoiado
desliza até a posição do parado (até ~15–30 cm em 0,3 s, conforme o pedaço). O "pé no chão" só corrige a altura,
não o deslizar no chão. Para um pé ficar fixo de verdade seria preciso IK no Blender (prender o pé de apoio e
deixar o outro fazer o passo). É mais trabalho; vale **só se o teste da 2ª versão mostrar o pé deslizando**.

## 7.3 Andar ↔ correr

**A ideia do plano ("os relógios na mesma fração do ciclo") está errada para o tatuado.** Medido no Blender
(pisada = o pé chega ao ponto mais baixo):

| | Andar: pisadas (fração do ciclo) | Correr: pisadas | Diferença |
|---|---|---|---|
| Mulher | esquerdo 0,28, direito 0,79 | esquerdo 0,33, direito 0,86 | +0,06 (mesma ordem: dá certo) |
| Tatuado (as do primeiro homem) | direito 0,03, esquerdo 0,55 | esquerdo 0,38, direito 0,86 | **+0,83** (na mesma fração, cairia o **pé trocado**) |

Portanto a troca precisa de **um deslocamento de fase por personagem** (correr = andar + d), medido no Blender e
guardado no perfil. Mesmo assim, as poses no mesmo pé não são iguais (a corrida é mais aberta e o tronco mais
inclinado), então sobra um pequeno salto de pose. Ainda assim é bem menor que hoje, porque hoje o relógio da
corrida anda sozinho e a troca cai em qualquer ponto.

**Atenção:** os tempos de passo do som do tatuado (`MAN_WALK.steps = [0,31; 0,84]`, ou 0,30 e 0,81 do ciclo) **não
batem** com a pisada medida agora (0,03 e 0,55). Foram medidos por métodos diferentes (a ponta do pé tocando o
chão, contra o tornozelo no ponto mais baixo). Antes de usar as fases, **é preciso conferir visualmente** qual das
duas medidas está certa. Se for a nova, o som do passo do tatuado está fora do tempo hoje.

## 7.4 Desempenho: quantas pessoas, e quanto custam

**O que acontece com cada pessoa a cada quadro (`place_one`):** o `set_world` do jogo (mover o nó) e o `update`
da instância (andar a animação), **mesmo escondida e mesmo com 0 segundos**. O desenho é que é limitado: o gancho
(`gather`) desenha só a pessoa mostrada, mais a de antes por 2 quadros. Então:
- **placa de vídeo:** escondidas não custam (não são desenhadas);
- **processador:** cada pessoa custa um `set_world` e um `update` por quadro, e **isso não foi medido**;
- **memória:** o modelo e as animações são carregados uma vez por arquivo, e cada pessoa guarda a própria pose
  (41 ossos) e o próprio estado. Deve ser pouco, mas também não foi medido.

**Quantas são hoje:** por personagem ativo, 10 (parado, andar, correr, pulo, pulo correndo, agachar, parar,
celular, celular andando e a transição `settle`). As "16 a mais" do plano original eram da 1ª versão; a 2ª pôs só
**1**. Em 1ª pessoa é o mesmo número, com o corpo sem cabeça (é um `Body` por vez, não os dois).

**Dá para reduzir sem perder nada:**
1. **Atualizar só quem é desenhado.** Cada pessoa guarda o tempo que deixou de andar (o relógio "devido"). Ao ser
   mostrada, recebe o `set_world` e o `update` com esse tempo acumulado antes de ser desenhada. O custo por quadro
   cai de 10 pessoas para 1–2, seja qual for o total. **Risco:** a piscada de antes era o primeiro quadro da pessoa
   trocada sendo desenhado errado. Com instâncias "frias" esse risco aumenta, e precisa de teste (a sobreposição
   de 2 quadros deve cobrir).
2. **Juntar animações curtas numa só, como foi feito com o `settle`:** o parar, o pulo e o pulo correndo
   poderiam ser uma pessoa só, tocada por trechos. Isso tira 2–3 pessoas.
3. **Criar só na hora que precisa:** não convém. Criar uma pessoa leva tempo (o modelo precisa estar pronto), e o
   primeiro quadro sairia errado ou atrasado.

**Como medir (antes de decidir):**
- **No plugin:** cronometrar `person::place` (QueryPerformanceCounter) e escrever no log, a cada 10 s, a média e
  o pior quadro em microssegundos. Isso dá o custo direto das pessoas.
- **No jogo:** a pé, parado no mesmo lugar e na mesma hora, 1 minuto com as 10 pessoas e 1 minuto com só andar e
  parado (por uma chave de teste no plugin). Comparar os fps do app (`performance` no `app.log`) e a placa
  (nvidia-smi). Fazer depois que o notebook esfriar: ele baixa a frequência sozinho a 87 °C, e isso estraga a
  comparação.

## 7.5 Casos que o plano não tratava

| Caso | O que acontece | O que fazer |
|---|---|---|
| Apertar e soltar o W rápido (menos de 0,15 s) | A confirmação ignora: continua andando. | Nada. |
| Parar e voltar a andar logo | Transição cortada; o andar volta da fase de onde saiu, com um pequeno salto (7.2). | Aceitável no protótipo. |
| Correr e parar | A transição só vale para o andar. Da corrida continua o "parar" antigo, com o salto de pose antigo. | Etapa 4: um `settle` da corrida, do mesmo jeito. |
| Shift (correr) durante a transição | Mover de novo corta a transição e mostra a corrida, com o relógio dela onde estiver: salto. | Resolvido junto com a 7.3 (a fase da corrida tirada da do andar). |
| Agachado andando | Não há animação de andar agachado: a pose de agachado desliza a 0,9 m/s. Não é a transição. | Uma animação "Crouch Walk" do Mixamo, se o dono quiser. |
| Pulo ou celular durante a transição | Cancelam a transição (já tratado). | Nada. |
| Entrar e sair do caminhão | O `Body` inteiro é solto ao subir (`hide`) e criado de novo ao descer. A transição não sobrevive à troca. | Nada; testar mesmo assim. |
| Mudar de personagem | Cada personagem tem o próprio `settle` e as próprias medidas (o tatuado é diferente da mulher). | Fases e velocidades sempre por perfil. |
| Modelo, animação e cápsula | O modelo é colocado no pé da cápsula a cada quadro (`on_feet`). A animação é *in place*. A cápsula manda na posição. Não há como o modelo "andar mais" que a cápsula, só as pernas. | É exatamente por isso que a sincronia (7.1) é o que resolve o escorregar. |

## 7.6 As transições no Blender

- **O rig:** as transições usam os mesmos ossos e o mesmo esqueleto do personagem: a marca do esqueleto (8 bytes)
  é igual em todos os arquivos, conferida a cada exportação. Rotação por quaternion (slerp, pelo caminho mais
  curto) e posição linear: é a forma correta de misturar poses.
- **A continuidade:** o primeiro quadro de cada pedaço é a pose exata do andar naquele oitavo do ciclo (avaliada
  nas curvas, inclusive entre quadros), e o último é o quadro 1 do parado. A suavização (*smoothstep*) começa e
  termina sem tranco.
- **Os pontos escolhidos:** com 8 pedaços a cada 1/8 do ciclo, o pedaço mais perto está sempre a 1/16 ou menos.
  Não precisa coincidir com uma pisada, porque a pose é congelada no ponto exato. Escolher só as pisadas (2 por
  ciclo) forçaria esperar, e foi isso que deu errado.
- **Os pés:** a altura é garantida, com o pé mais baixo no chão em todos os quadros. O deslize horizontal durante
  a mistura não é (7.2). A forma de conferir é renderizar cada pedaço de lado com uma grade no chão, ou medir
  quanto o pé apoiado anda em cada pedaço, pelo mesmo cálculo da velocidade do pé.

## 7.7 Proposta revisada

**Pode ficar:**
- o diagnóstico (seções 1 a 3): o motor não mistura animações; transição só como animação pronta; relógio
  controlável por pessoa;
- a 2ª versão da transição andar → parado (pose congelada, 8 pedaços, uma pessoa só), **se passar no teste do
  dono**;
- a sincronia pela velocidade da cápsula, **com as mudanças abaixo**.

**Precisa mudar:**
1. A sincronia: velocidade **filtrada**, fator com **piso (~0,5) e teto (~1,3)**, e velocidade natural **por
   animação e por personagem**. Com o celular, limitar a cápsula a 1,1 m/s em vez de acelerar as pernas.
2. Andar ↔ correr: **deslocamento de fase por personagem** (+0,06 a mulher, +0,83 o tatuado), depois de conferir
   as pisadas visualmente.
3. Medir a corrida do tatuado de novo (pelo comprimento do passo), e o dono decidir entre pernas mais lentas,
   correr mais rápido ou outra animação.
4. O desempenho medido antes de pôr mais pessoas, e "atualizar só quem é desenhado" como a forma de crescer sem
   custo.

**Riscos:**
- a pose congelada em 0,3 s pode parecer "robótica": é só o teste que diz;
- o pé apoiado desliza durante a transição (sem IK);
- as duas medidas de pisada do tatuado discordam (som e fase podem estar errados);
- a aceleração da câmera de andar não foi medida;
- atualizar só quem é desenhado pode trazer a piscada de volta.

**Mais simples ou mais eficiente?** A transição como pedaços numa só animação é a forma mais barata possível
dentro do motor: uma pessoa e uma animação por personagem. Para os pés, não há nada mais simples que a sincronia
de velocidade, porque o motor não tem *root motion* nem IK em tempo real que possamos usar. A única "solução mais
simples" seria aceitar o escorregar, que é o problema a resolver.

**Testes pequenos, um por vez (cada um com a chave para voltar):**

| # | Teste | Mexe em | Como saber se passou |
|---|---|---|---|
| A | A 2ª versão do andar → parado, que já está instalada | nada novo | O dono: para sem passo a mais e sem pulo de pose? |
| B | Medir: a velocidade da cápsula quadro a quadro (apertar e soltar o W) e o custo de `place` | só logs no plugin | Números no `plugin.log` |
| C | Celular: cápsula a 1,1 m/s com o celular aberto | um limite no perfil | Os pés não escorregam andando com o celular |
| D | Sincronia só do andar (fator filtrado, piso e teto) | relógio do andar | Pés parados no chão andando, arrancando e freando |
| E | A mesma sincronia na corrida, com a decisão da 7.1 | relógio da corrida | Igual, correndo |
| F | Andar ↔ correr com a fase por personagem | relógio da corrida na troca | A troca cai no mesmo pé, sem salto |
| G | Correr → parado (um `settle` da corrida) | animação nova + plugin | Igual ao A, correndo |
| H | Atualizar só quem é desenhado (se B mostrar custo) | `place` | O custo cai e não volta a piscar |

**Primeiro protótipo recomendado:** **A** (é só o dono testar o que já está instalado), junto com **B** (só logs,
sem mudar nenhum comportamento). Depois, **C**: é o caso mais grave medido (o corpo 45% mais rápido que os pés
com o celular), e a mudança é a menor de todas (um número). Com os números de B, a **D** fica com piso e teto
decididos por medida, não por palpite.

## 7.8 Resultado do teste B (2026-11-06, 19:44–19:47, ETS2, a mulher)

Os registros `measure:` do `plugin.log` (`crates/plugin/src/character/meter.rs`), a pé, com a mulher. O tatuado não
entrou neste teste. Foram 28 paradas, ~25 arrancadas e 2.776 quadros andando.

**1. O custo das pessoas é desprezível.** 10 pessoas criadas: **23 µs por quadro em média** (0,14% de um quadro
de 16,7 ms), pior quadro entre 50 e 317 µs, estável em 21 janelas de 10 s. **O teste H (atualizar só quem é
desenhado) não é necessário.** Dá para ter mais pessoas, desde que se continue medindo.

**2. A cápsula anda mais rápido que o limite.** A física diz 1,6 m/s, que é o nosso limite (`WALK_SPEED`), mas a
posição avança **~2,15 m/s** (2,1–2,2 em 1.564 quadros). Correndo, a física diz 3,2 e a posição avança **~3,78**.
A diferença é quase a mesma nos dois casos (+0,55 e +0,58 m/s), como se o próprio jogo somasse um movimento além
da velocidade da física. **A causa não foi encontrada:** pode ser a câmera de andar movendo o próprio nó, ou a ordem
em que o limite é aplicado. **É o que o corpo visível faz** (o modelo é posto na posição da cápsula a cada quadro).
Então a tabela da seção 3 estava errada para baixo. O correto é:

| Mulher | Pé no chão (animação) | Corpo de verdade | Diferença |
|---|---|---|---|
| andar | 1,49 m/s | **2,15** | o corpo **44%** mais rápido |
| correr | 3,27 | **3,78** | 16% |
| andar com celular | 1,10 | **2,15** | **95%** (quase o dobro) |

Isso explica o escorregar ao andar melhor que qualquer outra hipótese.

**3. Arrancar e parar são quase instantâneos.**
- **Arrancando**, a velocidade chega ao máximo em **0,07 s** (4 quadros) andando e em 0,10 s correndo.
- **Parando, a cápsula para em 1 quadro** (0,016 s): a física vai a 0 no mesmo quadro, sem frear.
- O personagem só passa a contar como "parado" **0,15–0,17 s depois** (`STILL_AFTER`). Nesse tempo o andar
  continua dando passos com o corpo já parado: é o escorregar ao parar, e ele atrasa a transição pelo mesmo tempo.

**4. Não há quadros "sem física" andando.** Dos 2.776 quadros andando, os 78 com velocidade zero estão todos dentro
desses 0,15 s depois de soltar a tecla. Andando de verdade, a velocidade não cai a zero. A razão da espera de 0,15 s
(a piscada de antes, com a velocidade oscilando ao parar) **não aparece mais nas medidas**. Aparecem ainda picos de
40–75 m/s: são saltos de posição, como descer do caminhão, e precisam ser ignorados por qualquer uso futuro.

### O que muda no plano

- **7.1 (sincronia pela velocidade):** como a velocidade é praticamente "zero ou cheia", não vale a pena um relógio
  que acompanha a aceleração (filtro, piso, teto). **O que resolve é casar a velocidade cheia de cada animação com
  a do corpo.** Há dois jeitos:
  - **(a) baixar o corpo até a velocidade da animação.** O limite passa a ser regulado pela velocidade medida de
    verdade, e não pela da física (que mente). A mulher andaria a 1,49 m/s, correria a 3,27 e andaria com o celular
    a 1,10: os pés ficam exatos, mas ela fica **mais lenta** que hoje (andar de 2,15 para 1,49);
  - **(b) acelerar a animação até o corpo.** O andar tocaria 1,44× (passos visivelmente apressados) e o celular
    1,95× (inaceitável).
  - O recomendado é **(a) para andar e para o celular, e (b) só na corrida** (1,16×, imperceptível). **A velocidade
    é decisão do dono**: o andar ficaria ~30% mais lento que o atual.
- **7.2 (parar):** baixar `STILL_AFTER` de 0,15 para ~0,05 s (3 quadros). A transição começa quase junto com o corpo
  e sobram no máximo 3 quadros de passo. É um número, e se a piscada voltar, volta-se o número.
- **7.4 (desempenho):** resolvido. Não há custo que justifique economia de pessoas.

### Próximos testes revisados

| # | Teste | Mexe em | Decisão do dono antes |
|---|---|---|---|
| A | A 2ª versão do parar (instalada) | nada | — |
| C' | Parar mais cedo: `STILL_AFTER` 0,15 → 0,05 s | um número | — |
| D' | Celular: o corpo a 1,1 m/s de verdade, com o celular aberto | o limite, medido pela posição | — |
| E' | Andar: o corpo na velocidade da animação (1,49 a mulher) | o mesmo limite | sim: andar mais devagar |
| F' | Correr: a animação 1,16× (a mulher) | o relógio da corrida | — |
| G | Andar ↔ correr com a fase por personagem (7.3) | relógio da corrida | — |
| — | Repetir B com o tatuado (as velocidades dele são outras) | nada | — |

O primeiro recomendado agora é o **C'**: é o que mais aparece ao parar e é o menor possível. Depois o **D'**,
porque com o celular o corpo vai a quase o dobro da velocidade dos pés.

## 7.9 Hipótese: cada animação começa num ponto aleatório (2026-11-06, depois do C')

O dono, depois do C': "quando ela para ainda dá um passinho para frente" e, no pulo parado, "o cilindro pula, aí
ela abaixa, o cilindro volta, e aí ela faz a animação". O log da mesma sessão mostra o plugin certo: Espaço às
91,62 s, a física saiu do chão 0,47 s depois (a decolagem da animação no relógio do plugin) e ficou 0,65 s no ar.
Então **o que o jogo desenha não está no ponto que o plugin pensa**.

O plugin conta o tempo de cada pessoa a partir de 0 quando a cria (`Person::clock`). Para mandar uma animação a um
quadro (a decolagem, o pedaço do `settle`, o quadro 1 do parado), ele soma o tempo que acha que falta. Se o jogo
começa a animação de cada *mover* num ponto próprio (é comum, para uma multidão não andar em passo), toda conta
erra por um valor diferente em cada pessoa. Isso explica o pulo atrasado e o passo ao parar: o `settle` tocaria
um trecho que atravessa de um pedaço para a pose do andar do seguinte. E explica também por que o pulo nunca
sincronizou, apesar de todas as correções.

**Teste (só log):** `crates/plugin/src/character/probe.rs` lê a memória da instância do parado e da corrida nos 16
primeiros quadros e escreve no log (`probe:`) os valores que avançam exatamente com o tempo mandado, e de onde
começaram. Se começarem longe de 0, a hipótese está confirmada, e a correção é ler (ou escrever) esse tempo em vez
de confiar na conta do plugin.

## 7.10 O que os vídeos do dono mostram (2026-11-06, 20:16–20:22)

Analisados quadro a quadro (30 fps) no Blender: `Gravando ets animação 2026-11-06 .mp4` (107 s) e `Gravando animação
agachado 2026-11-06.mp4` (68 s), em Downloads. O `plugin.log` é da mesma sessão. O 1º pulo parado (Espaço às 252,62 s
no log) começa no quadro 870 do vídeo, e a física ficou no ar entre os quadros 884 e 904.

- **Pulo parado:** a animação toca inteira e em ordem: agacha (quadros 872–891), estica com os braços para cima
  (893–910), pousa e volta. **Mas entre os quadros 884 e 904, quando o log diz que a física estava no ar, nem os
  pés nem a câmera de 3ª pessoa sobem: o chão fica parado na imagem.** Antes do pulo (820–870) também nada se mexe.
  Então, na 3ª pessoa, o corpo não acompanha a altura do pulo. Não é atraso da animação: falta subir o corpo junto.
  Como o "pé no chão" (`pe_no_chao.py`) tirou a subida da própria animação, o pulo parece "no lugar".
  Hipótese: a posição da câmera de andar (`PLACEMENT`), de onde o corpo é posto, não sobe com a cápsula no pulo. A
  confirmar com um log por quadro da altura dela e da velocidade vertical durante um pulo.
- **Agachado andando:** fica na pose de baixo do "Air Squat Bent Arms" e desliza (não há animação de andar
  agachado). **Os braços abertos são dessa animação**, um exercício de agachamento com os braços à frente. Em 1ª
  pessoa as mãos ficam abertas nos dois cantos da tela. Precisa de animações de agachar de verdade (Mixamo):
  *Crouch Idle* (parada agachada), *Crouched Walking* (andar agachado, In Place) e, se der, *Standing To Crouched* e
  *Crouch To Standing*.
- **Parado e andando:** os braços do parado estão normais (ao lado do corpo).

## 7.11 A altura do pulo, medida (2026-11-06, 21:00)

`measure: jump height` (plugin, `meter.rs`): em 4 pulos parados, a posição de onde o corpo é posto (a câmera de
andar, `PLACEMENT`) **sobe 0,434 m e volta em 0,60 s**, uma parábola perfeita, igual nos 4. A velocidade vertical
vai de +3,0 a −2,7 m/s. Então a física pula, e a posição que o plugin usa para o corpo sobe junto.

O vídeo das 20:16 (mesmo código do pulo) mostrava o corpo e a câmera de 3ª pessoa parados durante o pulo. As duas
coisas só se explicam se **o jogo prende a pessoa (o *mover*) ao chão**, qualquer que seja a altura que o plugin dá
ao nó dela, ou se a gravação foi de um momento diferente do que alinhei com o log. Falta ver um pulo gravado com o
plugin atual: se ela não subir na imagem, é o jogo prendendo a pessoa ao chão, e a correção é achar como ele faz
isso.

## 7.12 A causa do pulo atrasado, pela descrição do dono (2026-11-06, 21:05)

O dono descreveu: "aperto pra pular, ela sobe [dura]; desce; aí ela faz a animação de agachar um pouquinho e o pulo
de novo, levantando as mãos". No pulo correndo, não: "quando aperto, ela já faz a animação". A diferença entre os
dois: o pulo correndo começa do quadro 1 da animação (o plugin quase não a avança), e o parado começa do quadro
15. Para isso o plugin manda **0,47 s de uma vez** à animação. Se o jogo só aceita parte de uma atualização longa,
a animação começa do quadro 1 (ainda em pé), e o corpo sobe "duro" na hora em que o plugin acha que ela já agachou.
Depois a animação chega ao agachar e ao pulo, "o pulo de novo". O mesmo explica o passinho ao parar: a transição
também é mandada de uma vez para o pedaço certo.

**Mudança:** `place_one` (`person.rs`) manda toda atualização maior que 1/30 s em **pedaços de 1/30 s**, quantos
forem precisos, no mesmo quadro (`STEP`). A atualização normal de cada quadro continua inteira. O teste dos gêmeos
(`probe::Twins`) ainda manda 1 s inteiro a um deles, de propósito, para confirmar no log.

## 7.13 Por que o pulo parecia "falso": a câmera subia junto (2026-11-06, 21:32)

A gravação do dono (`Gravando 2026-11-06 213240.mp4`, Downloads), pulo parado com o "Jump" cortado sem o agachar do
começo, analisada quadro a quadro, com a borda entre a grama e a estrada medida em cada quadro: ela desce alguns
pixels e volta entre os quadros 428 e 444. Ou seja, **a câmera de 3ª pessoa sobe e desce junto com o pulo**. Na
tela, a personagem fica no mesmo lugar e só o mundo mexe um pouco. As pernas encolhidas no ar parecem um agachar no
chão, e logo depois vem o agachar do pouso: as "duas animações" que o dono via. O pulo acontecia (a física sobe 43 cm,
7.11), só não aparecia.

**Mudança:** na 3ª pessoa a câmera fica na altura de onde ela saiu do chão enquanto está no ar (`jump_ground` em
`character.rs`), descendo junto só se ela cair mais baixo (de um degrau). Assim, ela sobe na tela. Na 1ª pessoa a
vista continua subindo com o pulo.

Ainda a ver: no fim do pulo, a troca para o parado mostrou a personagem dobrada por 2 quadros (agachada e de pé).
É a sobreposição de 2 quadros (`DRAWN_BEFORE`) com poses muito diferentes, e indica que a animação do pulo ainda
estava agachada quando o plugin achou que tinha acabado (~0,3 s de diferença).

## 7.14 O "pulinho" no fim do pulo (2026-11-06, 22:00)

O dono: no fim do pulo, já no chão, "ela dá um pulinho pra cima no último frame", "como se resetasse a animação:
abre os braços e volta pro parado". Duas causas, as duas corrigidas:
1. **A animação dava a volta.** No último quadro, o plugin mandava o pulo um pouco além do fim. O jogo repete a
   animação em laço, então ela voltava ao quadro 1 (o impulso, de braços abertos), e a sobreposição de 2 quadros da
   troca (`DRAWN_BEFORE`) desenhava essa pose. Agora o pulo nunca passa do fim (`END_MARGIN`).
2. **O fim não era o parado.** O pulo terminava meio dobrado e o parado entrava em pé, no ponto em que o relógio
   dele estivesse. Agora o pulo vai até o fim original (quadro 79, já de pé), e 8 quadros novos levam essa pose ao
   quadro 1 do parado (`tools/personagem/emendar_no_parado.py`). Quando o pulo acaba (parado, não andando), o parado
   começa do quadro 1 (`stand_by`).
