# Manual do MaxDevMod

O MaxDevMod (antigo WestPlay) coloca uma central multimídia no estilo CarPlay na tela do caminhão, no **American Truck
Simulator** e no **Euro Truck Simulator 2**. Ele também traz um celular dentro do jogo, a câmera de ré e o Modo
Personagem, para descer do caminhão e andar a pé.

Este manual vale para a versão **2.4.2 beta**. O que tem a marca **(beta)** é novo e ainda está em teste: pode ter
falhas. Se algo não funcionar, veja a parte [7. Problemas comuns](#7-problemas-comuns).

**Sumário**

1. [Começando](#1-começando)
2. [A tela do caminhão](#2-a-tela-do-caminhão)
3. [O celular do jogo](#3-o-celular-do-jogo)
4. [Modo Personagem (beta)](#4-modo-personagem-beta)
5. [Volante pelo celular (beta)](#5-volante-pelo-celular-beta)
6. [Teclas e atalhos](#6-teclas-e-atalhos)
7. [Problemas comuns](#7-problemas-comuns)

---

## 1. Começando

### Instalar

1. Feche o jogo.
2. Baixe o instalador na página de [versões](https://github.com/Mateuslima09/ETS-ATS-CarPlay/releases) e rode.
3. O app do MaxDevMod abre e guia você: acha o jogo, instala o plugin e o mod de telas e monta o seu mapa. A montagem
   do mapa só acontece com o jogo fechado.
4. Pronto. Abra o jogo e entre num caminhão.

**Requisitos:** Windows 10 ou 11 (64 bits) e o ATS e/ou o ETS2 na versão 1.61 (outras podem funcionar; o app avisa
quando uma não foi testada).

### O aviso do Windows

O instalador não é assinado. Se aparecer a tela azul "O Windows protegeu o computador", clique em **Mais
informações** e depois em **Executar assim mesmo**.

### Abra o app antes do jogo

O app do MaxDevMod no PC é quem faz tudo funcionar. **Abra o app primeiro e depois o jogo**, e deixe o app aberto
enquanto joga. Sem ele, a tela, o celular e o Modo Personagem (F8) não funcionam.

### Os dois jogos (ETS2 e ATS)

- O app configura primeiro o jogo que encontrar.
- Para usar também no outro: no app, vá em **Configuração → Outros jogos** e clique em **Instalar**.
- Configurado nos dois, o MaxDevMod troca sozinho para o jogo que você abrir.

### As páginas do app no PC

| Página | Para que serve |
|---|---|
| **Tela do caminhão** | Liga e desliga o MaxDevMod, mostra se o jogo está conectado e tem os QR Codes do celular e do volante. |
| **Controles** | A tecla de usar a tela, a velocidade do ponteiro e todos os atalhos. |
| **Personagem** | O Modo Personagem, quem anda, a sensibilidade do mouse e as partes que ligam e desligam. |
| **Personalizar** | Seus papéis de parede e ícones. |
| **Contas** | Entrar no YouTube, Spotify, Apple Music e TikTok. |
| **Mapa** | O seu mapa, feito a partir do seu jogo. |
| **Configuração** | Onde está instalado, **Reinstalar**, **Desinstalar** e **Outros jogos**. |
| **Sobre** | Sua versão e um relatório para copiar se algo der errado. |

---

## 2. A tela do caminhão

### Onde ela aparece

- **Na tela do próprio caminhão**, nos caminhões que têm tela no painel (Scania S e R, DAF 2021, Volvo FH, Cascadia,
  T680, VNL e outros) e nos caminhões de mod com tela de GPS. Não precisa fazer nada.
- **No tablet do MaxDevMod**, nos outros caminhões: coloque o tablet na oficina, em Acessórios.
- **Na tela flutuante** (beta), em qualquer caminhão: veja abaixo.

### Os apps da tela

Mapas (com a rota e a busca de destino, que manda a rota para o GPS do jogo), Painel, Música (Apple Music, Spotify e
YouTube Music), YouTube e Ajustes. Para entrar nas suas contas de música, use **Contas** no app do PC.

### Usar com o mouse

1. **Segure Alt esquerdo** (a tecla de interação).
2. Mexa o mouse: uma **bolinha** aparece na tela do caminhão e segue o mouse.
3. Clique para tocar nos botões.
4. Solte o Alt para voltar a olhar em volta da cabine.

Dicas:

- Prefere não segurar? Em **Controles**, ligue **Pressionar uma vez em vez de segurar**: aperte Alt para usar a tela e
  aperte de novo para sair.
- A tecla pode ser trocada em **Controles → Segure para usar a tela**.
- O tamanho da bolinha muda no celular do jogo: **Ajustes → Bolinha do mouse** (Pequena, Normal, Grande, Muito
  grande).
- A tela ficou torta ou cortada num caminhão de mod? Use **Ajustar Tela** nos Ajustes da tela, ou pelo celular em
  **Ajustes → Tela do veículo** (bordas, proporção, tamanho e formato).

### A tela flutuante (beta)

Uma tela do MaxDevMod "flutuando" na cabine de qualquer caminhão, sem acessório.

1. No celular do jogo, abra o app **Tela**.
2. Ligue a **Tela flutuante**.
3. Use **Mover**, **Girar** e **Tamanho** para colocar ela no lugar. Cada caminhão guarda o seu lugar.
   **Voltar ao lugar padrão** desfaz tudo.

Dicas: para diminuir, use **Tamanho**, não "Frente" (empurrada para frente, ela fica atrás do painel). Com o caminhão
desligado, a tela fica preta, como a do caminhão, e acende de novo quando você liga a parte elétrica. Ela ainda não
some atrás de objetos da cabine.

### A câmera de ré e os sensores (beta)

- Engate a **ré**: a tela vira câmera de ré, na traseira da carreta (se tiver uma engatada) ou do caminhão.
- **Linhas guia** no chão: vermelha (1 m), amarela (2 m) e verde (3 m). Elas curvam com o volante.
- **Sensores de estacionamento**: o bipe fica mais rápido e mais alto conforme você chega perto de algo, e vira um tom
  contínuo quando é para parar. A distância aparece ao lado da imagem.
- A imagem é desenhada a partir do mapa do seu jogo; não é a imagem do próprio jogo.
- Ela some pouco depois de você tirar a ré, ou na hora se andar para frente.
- Para ligar ou desligar cada parte (câmera, bipe, linhas): **Ajustes → Câmera de Ré**, na tela do caminhão.
- Para mudar a posição da câmera: app **Câmera de Ré** no celular do jogo (distância, altura, inclinação, giro e campo
  de visão). Melhor ajustar de ré, olhando a tela da cabine.

---

## 3. O celular do jogo

### Abrir e usar

- **Alt+4** tira e guarda o celular.
- **Segure Alt** e use o mouse para mexer nele, como na tela do caminhão.
- Ele se guarda sozinho quando você abre o menu do jogo.
- Os apps têm duas páginas: use as setas nos cantos para trocar.

### No seu celular de verdade

O mesmo celular também abre no seu celular de verdade:

1. Deixe o celular na **mesma rede Wi-Fi** do PC.
2. No app do PC, abra **Tela do caminhão**.
3. Escaneie um dos QR Codes com a câmera do celular: um abre o celular do jogo, o outro abre a tela do caminhão
   inteira (com o celular deitado).

### Os apps

| App | O que faz |
|---|---|
| **Mapas** | O mapa com a rota. |
| **Música**, **YT Music**, **Spotify** | O que está tocando, com volume e controles. |
| **Comandos** | Vidros, giroflex, pisca-alerta, eixo, luz da cabine, buzinas, freio de mão, carreta, ignição e motor. Os vidros sobem e descem enquanto você segura o botão. |
| **Frete** | A carga, o peso, de onde para onde, quanto falta e o prazo. |
| **Caminhão** | Velocidade, diesel, autonomia, piloto automático, motor, freio de mão e faróis. |
| **Suspensão** (beta) | A suspensão a ar do caminhão e da carreta (veja abaixo). |
| **Câmera de Ré** (beta) | Ajusta a posição da câmera de ré. |
| **Tela** (beta) | A tela flutuante. |
| **Nupay** | O saldo da sua conta no jogo: fretes, multas, pedágios, balsas e combustível. Em dólar, real ou euro. |
| **Serviços** | Abre a janela de serviços do jogo (guincho, oficina) e estaciona rápido. |
| **Personagem** | Escolhe quem desce do caminhão, pela foto. Abre direto com **Alt+5**. |
| **TikTok** | O site do TikTok: feed, curtir, seguir, comentar. |
| **Ajustes** | As opções do celular (veja abaixo). |

### Suspensão (beta)

O desenho do caminhão mostra a altura de cada mola ao vivo.

- Escolha o que mexer: **Caminhão**, **Dianteira**, **Traseira** ou **Carreta**.
- **Subir** e **Baixar** mexem enquanto você segura. **Máxima**, **Nivelar** e **Mínima** vão direto.
- Só funciona com o **motor ligado**. Desligado, o app avisa (ligue em Comandos → Ignição).
- Cada caminhão sobe e desce até onde a suspensão a ar dele vai.

**Suspensão automática** (ligue no próprio app):

- Um tempo acima de 80 km/h, ela baixa um pouco; abaixo de 60 km/h, volta ao normal.
- Em estrada ruim, ela sobe e fica em cima até a estrada ficar lisa um bom tempo.
- Freando forte por um tempo, ela sobe um pouquinho e volta quando você solta o freio.
- Engatando uma carreta pesada parado, ela sobe a traseira de volta para onde estava.
- Mexer você mesmo pausa a automática por 1 minuto.
- Funciona melhor com **Voltar ao normal sozinha** desligado.

Outras opções do app (valem na próxima vez que o jogo abrir):

- **Frente mais baixa que o jogo**: Normal, Baixa ou Máxima.
- **Amortecedor**: Macio, Firme ou Muito firme.
- **Voltar ao normal sozinha**: ligado, o jogo volta a suspensão para a altura normal quando você anda.

### Ajustes do celular

- **Aparência** (Auto, Claro, Escuro), **Unidades** (Km ou Milhas), **Moeda do Nupay**, **Papel de parede**,
  **Volume**.
- **Tela do veículo**: o Ajustar Tela pelo celular.
- **Bolinha do mouse**: o tamanho dela.
- **Espelhos virtuais**: Nenhum, Esquerdo, Direito ou Os dois.
- **Tela ligada (caminhão desligado)**: deixa a central ligada com o caminhão desligado.
- **Nota fiscal e engate manual**, **Abastecimento manual** e **Pagar no frentista**: as partes do Modo Personagem.
- **Notificações do jogo** e **Notificações da LIVE do TikTok**.

### Notificações (beta)

- **Do jogo**: pagamentos recebidos e feitos, multas, combustível (reserva, pagar o frentista), dano no caminhão,
  descanso e fretes. Cada grupo liga e desliga em **Ajustes → Notificações do jogo**. Use **Testar notificações do
  jogo** para ver como ficam.
- **Da LIVE do TikTok**: entre no TikTok em **Contas → TikTok**, no app do PC. Enquanto você faz live, quem segue,
  manda presente, compartilha ou se inscreve aparece no celular, com som.
- Com o celular guardado, só o topo dele sobe no canto da tela para mostrar o aviso.

---

## 4. Modo Personagem (beta)

Desça do caminhão e ande a pé, em primeira ou terceira pessoa. Completo no ETS2; no ATS ainda falta uma parte.
**Só no single player** (não funciona no TruckersMP), e o app do PC precisa estar aberto.

Para ligar ou desligar: app do PC → **Personagem → Modo Personagem**. Ligar ativa o modo desenvolvedor do jogo: vale
a partir da próxima vez que o jogo abrir.

### Descer e subir

1. Pare o caminhão e puxe o **freio de mão**.
2. Aperte **F8** para descer.
3. Para subir: vá até a porta do motorista e aperte **F8**. De qualquer lugar: **segure F8 por 2 segundos**.

A tecla F8 pode ser trocada em **Personagem → Tecla de descer e entrar**, no app do PC.

### Andar

| Tecla | Ação |
|---|---|
| **W A S D** | Andar |
| **Shift** | Correr |
| **Espaço** | Pular |
| **Ctrl** | Agachar |
| **V** | Trocar entre 1ª e 3ª pessoa (a câmera lembra a sua escolha) |
| **-** e **=** | Mouse mais lento / mais rápido |
| **E** | Interagir (conferente, quinta roda, pés da carreta, tanque, frentista) |

- Uma **barra de teclas** embaixo da tela mostra o que dá para fazer naquele momento. **Alt+6** esconde e mostra.
- O celular vai na mão quando você está a pé.
- Se um carro atropelar você, a tela escurece e você volta para o lado do caminhão.
- Quem anda: escolha pela foto em **Personagem** (no app do PC ou no celular do jogo, Alt+5). Vale na próxima vez que
  você descer.
- A lanterna ainda não está disponível nesta versão.

### Nota fiscal com o conferente (beta)

Só em frete com o **seu caminhão** (o frete rápido continua como sempre).

1. Na coleta, um **conferente** espera perto da carreta.
2. Desça (F8), chegue nele e aperte **E**.
3. A nota fiscal aparece grande na tela. Assine com o mouse (segure o botão e desenhe). **Limpar** apaga.
4. Clique em **Assinar**. O conferente carimba. Clique em **Coletar**.
5. Só agora o engate é liberado.
6. Na entrega, pare: o destinatário aparece ao lado da carreta. Aperte **E** nele, ele assina e carimba o canhoto, e
   você coleta. Só então dá para finalizar a entrega.

### Engate à mão (beta)

A tecla de engatar (T) não engata mais de dentro da cabine. Os passos são como na vida real:

**Para engatar:**

1. Encoste a quinta roda sob o pino da carreta e desça.
2. Vá até a quinta roda e aperte **E**.
3. **Trava**: segure o puxador vermelho com o mouse e arraste **para dentro** até o fim. Ele fica verde com um "clack".
4. Vá até os **pés da carreta** (na lateral dela) e aperte **E**.
5. **Manivela**: segure a manopla vermelha e gire em círculos com o mouse, **4 voltas no sentido anti-horário**, para
   subir os pés. A carreta engata.

**Para desengatar:**

1. Desça, vá até os pés da carreta e aperte **E**.
2. Gire a manivela **4 voltas no sentido horário** para baixar os pés.
3. Vá até a quinta roda, aperte **E** e arraste o puxador **para fora** até o fim.
4. Saia com o caminhão.

Dicas: soltando o puxador antes do fim, ele volta. O **×** fecha sem fazer nada. Se você sair andando com a trava
fechada mas sem subir os pés, a trava abre de novo.

Liga e desliga em: celular do jogo → **Ajustes → Nota fiscal e engate manual** (ou na página Personagem do app).

### Abastecimento à mão e pagamento (beta)

Na bomba, o Enter dentro da cabine não abastece mais.

1. Pare na bomba e desça.
2. Vá até o tanque, ao lado do caminhão, atrás da cabine.
3. **Segure E (ou Enter)** para abastecer. **Solte** para parar. Se encher, para sozinho com "Tanque cheio".
4. Depois aparece o aviso para pagar. O **frentista** espera na frente do caminhão.
5. Chegue nele e aperte **E**: o Nupay abre com litros, preço e total. Clique em **Pagar**.
6. O pagamento entra no histórico do Nupay como **Posto de combustível**.

Se você sair sem pagar, o jogo avisa "Você saiu sem pagar o abastecimento!" (o jogo já descontou o diesel ao encher).

Liga e desliga em: celular do jogo → **Ajustes → Abastecimento manual** e **Pagar no frentista**. Com o pagamento
desligado, já fica pago.

---

## 5. Volante pelo celular (beta)

Use o seu celular de verdade como volante: você gira o celular e o caminhão vira.

### Conectar

1. Celular e PC na **mesma rede Wi-Fi**.
2. No app do PC, abra **Tela do caminhão** e escaneie o QR Code **Volante**.
3. Na primeira vez, o navegador avisa sobre o certificado. Toque em **Avançado → Continuar**: é um certificado que o
   seu próprio PC criou.
4. Toque em **Tocar para começar**. No iPhone, aceite a permissão de movimento.
5. Segure o celular **deitado**, como um volante, com a tela virada para você.
6. O ponto fica verde com **Conectado**.

**iPhone ficou em "Conectando…" para sempre?** Abra no Safari o endereço que aparece na página, trocando o final por
`/volante.cer` (fica `https://<ip do PC>:47801/volante.cer`). Instale o perfil em **Ajustes → Geral → VPN e
Gerenciamento de Dispositivos** e ative a confiança em **Ajustes → Geral → Sobre → Ajustes de Confiança de
Certificados** ("MaxDevMod Volante (este PC)"). Depois recarregue a página.

### Calibrar

- **Centralizar**: zera a direção na posição em que o celular está agora.
- Virou ao contrário? **Ajustes → Inverter direção**.

### Pedais e botões

- **Acelerador** à direita e **Freio** à esquerda. Tocar mais **alto** no pedal dá mais força. Dá para usar os dois
  ao mesmo tempo.
- Embaixo do volante: **Ligar/Desligar** (segure Ligar para dar a partida), a marcha e **R**, **N**, **D** (câmbio
  automático do jogo), setas, pisca-alerta, freio de mão, **Farol**, **Alto**, **Ignição**, **Câmera ⟲** e **Câm.
  segue**.
- Botão **⛶** no topo: tela cheia.

### Ajustes

Toque em **Ajustes** na página do volante:

- **Direção**: sensibilidade, giro do celular até o fim, suavização, zona morta, curva, inverter direção e **Direção
  no jogo** (deixe em **Teclado/controle**, o padrão).
- **Pedais**: modo dos pedais (**Altura do toque**, **Inclinar o celular** ou **Liga/desliga**), sensibilidade, curva,
  largura e altura dos pedais.
- **Vibração** e **Acessibilidade** (canhoto, botões grandes, alto contraste, preencher a tela).
- **Voltar ao padrão** fica no topo.

Segurança: se você bloquear o celular, trocar de app, perder o Wi-Fi ou fechar o app do PC, o volante e os pedais
soltam na hora. O teclado e o volante de verdade continuam funcionando.

---

## 6. Teclas e atalhos

Os atalhos com Alt são o Alt esquerdo (a tecla de interação). Todos podem ser trocados em **Controles → Atalhos**, no
app do PC.

| Tecla | Onde | O que faz |
|---|---|---|
| **Segurar Alt** | Cabine | Usar a tela do caminhão e o celular com o mouse |
| **Alt+1** | Qualquer lugar | Diminuir volume |
| **Alt+2** | Qualquer lugar | Tocar / pausar |
| **Alt+3** | Qualquer lugar | Aumentar volume |
| **Alt+4** | Qualquer lugar | Tirar / guardar o celular |
| **Alt+5** | Qualquer lugar | Abrir o app Personagem no celular |
| **Alt+6** | Qualquer lugar | Esconder / mostrar a barra de teclas |
| **F8** | Parado, com freio de mão | Descer do caminhão |
| **F8** | Na porta do motorista | Subir no caminhão |
| **Segurar F8 (2 s)** | A pé, em qualquer lugar | Voltar para a cabine |
| **W A S D** | A pé | Andar |
| **Shift** | A pé | Correr |
| **Espaço** | A pé | Pular |
| **Ctrl** | A pé | Agachar |
| **V** | A pé | 1ª / 3ª pessoa |
| **-** / **=** | A pé | Mouse mais lento / mais rápido |
| **E** | A pé | Interagir: conferente, quinta roda, pés da carreta, frentista |
| **Segurar E (ou Enter)** | A pé, no tanque | Abastecer (solte para parar) |
| **Ré** | Dirigindo | Câmera de ré |

---

## 7. Problemas comuns

**O app não vê o jogo / a tela não aparece**

- Abra o **app do MaxDevMod antes do jogo** e deixe aberto.
- **Depois de atualizar o MaxDevMod, feche e abra o jogo de novo.** O app e o plugin do jogo precisam ser da mesma
  versão; com o jogo aberto, o plugin antigo continua carregado.
- Não abra o jogo "como administrador": o MaxDevMod não consegue falar com ele.
- Entre na cabine com a parte elétrica ligada. A página **Tela do caminhão** diz o que está faltando ("Esperando o
  jogo", "Entre no seu caminhão"...).
- O caminhão não tem tela? Coloque o **tablet do MaxDevMod** na oficina, ou use a **tela flutuante**.
- A tela está desligada? Em **Tela do caminhão**, confira **Mostrar o MaxDevMod**.
- Ainda não deu? **Configuração → Reinstalar** (com o jogo fechado).

**A tela fica preta com o caminhão desligado**

É normal. Para deixar ligada, use **Ajustes → Tela ligada (caminhão desligado)** no celular do jogo.

**O celular de verdade não conecta**

- Celular e PC na mesma rede Wi-Fi (não use os dados móveis).
- Escaneie o QR Code de novo na página **Tela do caminhão**.
- O Firewall do Windows pode pedir permissão para o MaxDevMod: permita na rede privada.
- Volante: aceite o aviso do certificado (veja a [parte 5](#5-volante-pelo-celular-beta)).
- Depois de atualizar o MaxDevMod: **Ajustes → Atualizar página** no celular.

**O F8 não faz nada**

- O app do PC precisa estar aberto.
- O caminhão precisa estar parado e com o freio de mão puxado.
- Não funciona no TruckersMP.
- Acabou de ligar o Modo Personagem? Feche e abra o jogo.

**Smart App Control (Controle Inteligente de Aplicativos)**

O Windows 11 pode ter o Controle Inteligente de Aplicativos ligado. Ele bloqueia alguns programas não assinados, como
o MaxDevMod, sem a opção "Executar assim mesmo". Se isso acontecer, avise o autor no
[GitHub](https://github.com/Mateuslima09/ETS-ATS-CarPlay) e use a versão anterior enquanto isso. Desligar esse
recurso é escolha sua: ele fica em **Segurança do Windows → Controle de aplicativos e do navegador**, e depois de
desligado o Windows não deixa religar sem reinstalar o sistema.

**Algo deu errado e quero ajuda**

No app do PC, abra **Sobre** e copie o relatório. Mande junto com o que aconteceu.
