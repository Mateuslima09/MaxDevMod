# Personagem próprio do WestPlay (do zero) e animação de entrar no caminhão

Pedido do dono (2026-11-05): um personagem **exclusivo nosso**, leve, com **animações novas** (parado, andar,
correr, pular, agachar) e uma **animação de entrar no caminhão**. Outras IAs (GPT, Gemini) podem ajudar; este
arquivo diz como, e traz no fim o texto pra colar nelas.

## A ideia em uma frase

Fazer o personagem e as animações no **Blender**, exportar pro formato do jogo com as **ferramentas oficiais da
SCS**, pôr tudo dentro do **mod do WestPlay** (`westplay.scs`, que o app já monta e o jogo já carrega) com
definições `mover_model` nossas, e o plugin mostra o nosso personagem **exatamente como hoje mostra o pedestre do
jogo** (`person.rs`: só muda o nome, ex.: `mover.wp_walk`; cada parte do nome com 12 caracteres no máximo).

Por que assim:

- **O jogo continua desenhando** (luz, sombra, noite, chuva), como hoje. Nada de desenhar por cima no DirectX.
- **Já fizemos algo parecido**: o tablet (`crates/setup/src/tablet.rs`) é um modelo de fora (CC BY 4.0)
  convertido pro `.pmd`/`.pmg` do jogo e posto no `westplay.scs`. O personagem é o mesmo caminho, com esqueleto e
  animações a mais.
- **Funciona nos dois jogos** (ETS2 e ATS): os formatos são os mesmos.

## O que existe pra usar

- **Blender** (grátis) + **SCS Blender Tools** (oficial da SCS, GPL): exporta modelo, esqueleto e animações no
  formato intermediário da SCS (`.pim`, `.pis`, `.pia`).
- **SCS Conversion Tools** (oficial): converte esse intermediário pro formato do jogo (`.pmd`, `.pmg`, `.pma`).
  O jogo usa `.pmd` (descrição), `.pmg` (malha), `.pma` (animação), `.tobj`/`.dds` (texturas), `.mat` (material).
- **Os `mover_model` do jogo** como molde (já lidos: `/def/world/mover*.sii`): cada um é um modelo, uma animação
  e os LODs, ex.:
  ```
  mover_model : mover.m_walk_06 {
      mover_name: "..."
      model_desc: "/model/mover/characters/models/generic/m_vanilla_ice_01.pmd"
      model_anim: "/model/mover/characters/animations/man/walk/walk_01.pma"
      lods[0]: "..._lod1.pmd"   lod_dist[0]: 50
  }
  ```
  O nosso seria igual, apontando pros nossos arquivos (`/model/westplay/character/...`), um `mover_model` por
  animação (parado, andando, correndo, pulando, agachado, entrando no caminhão), **todos o mesmo modelo**. O plugin
  já troca entre dois deles (parado/andando); é só aumentar a lista.

## Leve (não pesar o jogo)

- Corpo: **5 mil a 10 mil triângulos** (os pedestres do jogo são dessa ordem), com **2 ou 3 LODs** (metade e um
  quarto dos triângulos, a 30/60 m).
- **Uma textura** de 1024×1024 (cor), talvez uma de normal; um material só.
- Esqueleto simples: **~20 a 30 ossos** (quadril, coluna, pescoço, cabeça, ombros/braços/mãos, pernas/pés), sem
  dedos separados.
- Animações curtas e em laço: parado (2–4 s), andar (1 ciclo ≈ 1,1 s), correr (≈ 0,7 s), pular (subir, no ar,
  cair), agachado, **entrar no caminhão** (≈ 2–3 s, uma vez).
- Só existe **um** personagem na tela (o seu): o custo é pequeno, igual a um pedestre a mais.

## A animação de entrar (e sair) do caminhão

Como seria, com o que o plugin já sabe:

1. Você aperta a tecla perto da porta do motorista (o plugin já sabe onde fica a porta: `DOOR_OUT`, a posição do
   caminhão e o lado do motorista).
2. O plugin **leva o boneco pro ponto da porta**, virado pro caminhão (uns poucos quadros, deslizando), e toca a
   animação `wp_player_enter` (subir o degrau, segurar a alça, entrar).
3. No fim da animação, a câmera volta pra cabine (como hoje) e o boneco some.
4. Descer: a mesma ao contrário (`wp_player_exit`), começando na porta.

Limites a saber:

- **A porta do caminhão não abre** no jogo (os caminhões não têm porta animada no single player). A animação
  mostra o personagem subindo e "entrando" pela porta fechada, de costas pra câmera; na prática fica parecido com
  um corte de cena. Dá pra esconder com a câmera indo pra cabine um pouco antes do fim.
- **A altura do degrau muda** de caminhão pra caminhão: uma animação só serve pra uma altura média (≈ 1,2–1,5 m
  até o piso da cabine). Depois, se quiser, duas ou três versões (cabine baixa/alta).
- Isso vale também com os pedestres do jogo? **Não**: eles não têm animação de subir. Só com personagem próprio.

## Plano escolhido pelo dono (2026-11-05)

1. Fazer **só o modelo 3D** no Blender (com o GPT ajudando).
2. Pôr o esqueleto e as animações no **Mixamo** (site grátis da Adobe), com estas opções:
   - auto-rigger com **Skeleton LOD = "No Fingers"** (cerca de 25 ossos, dentro da meta);
   - nas animações de andar e correr, marcar **"In Place"** (o plugin é que move o boneco);
   - baixar em **FBX, 30 fps**, e a primeira vez **"With Skin"**.
3. Importar no Blender, conferir a escala (o Mixamo usa centímetros, o jogo usa metros) e exportar com as SCS
   Blender Tools.
4. Primeiro teste: **o modelo + a animação de andar**. Isso responde a pergunta que decide tudo: o jogo aceita um
   `mover_model` nosso com esqueleto de fora? Se aceitar, vêm as outras animações.
5. **Licença**: o Mixamo serve **para testar**. Antes de publicar, é preciso decidir, porque o `westplay.scs` é
   aberto e qualquer um extrai as animações dele. A Adobe permite usar o Mixamo em jogos, mas não permite
   redistribuir os arquivos soltos. A saída limpa, se for preciso, é trocar só as animações por outras CC0
   (por exemplo, a "Universal Animation Library" do Quaternius) mantendo o mesmo modelo.
6. **Subir na cabine: sem animação do corpo.** Decisão do dono: ao entrar, a câmera passa pra primeira pessoa e
   ela mesma "sobe" (porta → degrau → porta aberta → banco). Já está feito no plugin (`character/climb.rs`), sem
   precisar de animação no Blender nem no Mixamo. A seção "A animação de entrar" abaixo fica só como ideia antiga.

## Etapas (cada uma testável sozinha)

1. **Teste do caminho (pequeno, primeiro)**: um boneco simples (até um "manequim" de blocos) com **uma animação
   só** (andar), exportado com as ferramentas da SCS, dentro do `westplay.scs`, e o plugin mostrando
   `mover.wp_test_walk`. **Isso responde a pergunta que decide tudo**: o jogo aceita um `mover_model` de um mod
   com esqueleto nosso? (Esperado que sim: mods de pedestres existem.) Se não aceitar, fica o Plano B (desenhar
   nós mesmos, muito mais trabalho).
2. **O personagem de verdade**: modelo leve, textura, esqueleto, LODs; parado e andar.
3. **As outras animações**: correr, pular, agachar (o plugin já tem os momentos: correr com Shift, pulo, agachar).
4. **Entrar e sair do caminhão** (acima).
5. **Na página Personagem**: "WestPlay (próprio)" na lista "Quem anda", junto com os 32 do jogo.

## Licença (importante: o WestPlay é distribuído)

- **O melhor**: o modelo e as animações **feitos por nós** (ou pelas IAs, em scripts nossos), ou **CC0** (domínio
  público, ex.: os personagens e animações de "Quaternius", CC0). CC BY também serve, com o crédito no
  `THIRD_PARTY_NOTICES.md` (como o tablet).
- **Cuidado com Mixamo** (Adobe): pode usar em jogos, mas não pode redistribuir os arquivos como recurso; num mod
  aberto (`westplay.scs`, que qualquer um abre) fica no limite. Evitar.
- **Nada da SCS** dentro do nosso pacote (nem copiar o esqueleto ou as animações deles).
- Anotar tudo em `THIRD_PARTY_NOTICES.md` antes de publicar.

## Onde as outras IAs ajudam (e onde não)

- **Ajudam bem**: explicar e escrever **scripts Python do Blender** (criar o esqueleto, ajustar pesos, gerar
  animações simples de andar/correr por código, criar os LODs com "Decimate", organizar as "Actions" pra exportar);
  passo a passo das **SCS Blender Tools** e da conversão; revisar o `.sii` do `mover_model`; ideias de visual.
- **Geram imagens de referência** (frente/lado/costas do personagem, a "folha de personagem") pra modelar em cima.
- **Não fazem sozinhas**: um modelo 3D pronto, com esqueleto e animações bons, direto no formato do jogo. Isso é
  trabalho no Blender (alguém precisa abrir o Blender e seguir os passos, ou rodar os scripts que elas escreverem).
  Ferramentas de "texto/imagem → 3D" existem, mas a licença e a qualidade (esqueleto, triângulos) precisam ser
  conferidas caso a caso.
- **Eu (Claude) faço a parte do WestPlay**: o `mover_model` nos `.sii`, pôr os arquivos no `westplay.scs`, o plugin
  (trocar de animação, levar o boneco até a porta, a câmera), a página do app, o roteiro de teste.

## Texto pra colar no GPT ou no Gemini

> Estou fazendo um personagem 3D para um mod de Euro Truck Simulator 2 / American Truck Simulator (jogos da SCS
> Software). O personagem vai ser usado como "mover" do jogo (como os pedestres), carregado de um mod meu, e
> precisa de: (1) um modelo humano **leve** (5–10 mil triângulos, 2–3 LODs, 1 textura 1024×1024, um material),
> estilo realista simples, roupa de caminhoneiro (camiseta ou jaqueta, calça jeans, boné opcional); (2) um
> **esqueleto simples** (20–30 ossos, sem dedos separados); (3) animações em laço: **parado** (2–4 s), **andar**
> (ciclo de ~1,1 s, a 1,6 m/s), **correr** (ciclo de ~0,7 s, a ~3 m/s), **pular** (subir/no ar/cair) e
> **agachado**; e uma animação única de **subir na cabine de um caminhão pela porta do motorista** (~2–3 s, degrau
> até o piso da cabine a ~1,3 m, segurando a alça), e a mesma ao contrário para descer. Tudo precisa ser **meu ou
> CC0** (nada de Mixamo nem de assets da SCS), porque o mod é distribuído.
>
> Vou usar o **Blender** com as **SCS Blender Tools** (oficial) para exportar e as **SCS Conversion Tools** para
> converter para `.pmd`/`.pmg`/`.pma`. Me ajude passo a passo, como para um iniciante: (a) criar ou obter o modelo
> base (sugira modelos CC0 se for melhor partir de um); (b) montar o esqueleto e os pesos; (c) criar as
> animações (se der, com **scripts Python do Blender** que eu possa rodar); (d) fazer os LODs; (e) configurar e
> exportar com as SCS Blender Tools (um modelo, várias animações como "Actions"); (f) converter com as SCS
> Conversion Tools. A origem do modelo deve ficar nos pés, entre os dois pés, com o personagem olhando para −Z
> (a frente do modelo do jogo), 1 unidade = 1 metro, altura ~1,80 m. No fim, quero a lista de arquivos gerados e
> os caminhos dentro do mod (ex.: `/model/westplay/character/wp_player.pmd` e
> `/model/westplay/character/anim/walk.pma`).

Depois de pronto, o que mandar pra mim: os arquivos convertidos (`.pmd`, `.pmg`, `.pma`, `.tobj`, `.dds`, `.mat`)
ou o `.blend`, e de onde veio cada parte (pra licença). Eu ponho no `westplay.scs`, escrevo os `mover_model` e
ligo no plugin.
