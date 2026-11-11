# Porta abrindo, descer e subir a escada (função futura)

Pedido do dono (2026-11-11), a partir de um vídeo de outro plugin de ETS2: em terceira pessoa a porta do motorista
abre, o personagem desce os degraus com a porta aberta, e fecha a porta no chão. No vídeo funciona em qualquer
caminhão, inclusive mods que não foram preparados para isso.

Ainda **não começado**. Este arquivo guarda o plano, junto com o do abastecimento (`docs/abastecimento/PLANO.md`).

## São duas coisas separadas

### 1. O personagem descendo e subindo — dá para fazer

- Hoje o WestPlay já calcula o caminho chão → degrau → porta → banco (`crates/plugin/src/character/climb.rs`), mas
  move só a câmera e esconde o boneco, porque as pessoas do jogo não sabem subir.
- Os personagens agora são nossos: duas animações do Mixamo (descer e subir escada), ajustadas no Blender como o
  agachar (`tools/personagem/`), tocadas enquanto o boneco percorre esse mesmo caminho.
- Trabalho fino: o pé no degrau e a mão no puxador. A altura do degrau muda por caminhão: uma tabela por modelo,
  com um padrão para os mods.
- Os sons de abrir e de bater a porta já existem.

### 2. A porta abrindo — depende de uma investigação

Nos caminhões do jogo a porta não é uma peça separada: faz parte da cabine. Como no vídeo funciona em qualquer
caminhão, a **hipótese** (não confirmada) é que o outro plugin mexe na porta **na hora de desenhar**:

1. intercepta o desenho da cabine (fica entre o jogo e a placa de vídeo, onde o WestPlay já desenha o celular e a
   barra de teclas);
2. uma "caixa" invisível na região da porta do motorista escolhe os pontos do modelo que são a porta;
3. só esses pontos giram em torno de um eixo vertical na dobradiça; o resto da cabine fica parado;
4. o interior simples que todo caminhão tem para ser visto de fora pelos vidros aparece no buraco.

**Onde fica a porta em cada caminhão:** estimativa pela posição da cabine e da cabeça do motorista (já usada para o
ponto de descer), tabela para os modelos conhecidos, e um ajuste pelo jogador (como o "Ajustar Tela") para os mods.

**O que falta descobrir antes de prometer:**

- **Reconhecer a cabine** entre tudo o que o jogo desenha. A parte mais incerta.
- **O corte ficar limpo:** sem uma emenda do modelo na linha da porta, os triângulos da borda esticam. Nos
  caminhões originais costuma haver emenda; nos mods varia.
- **O lado de dentro da porta:** o modelo só tem a face de fora; por dentro ela pode sumir ou sair na cor da
  lataria.
- **Sombra e reflexo:** o jogo desenha o caminhão várias vezes (sombra, espelhos); a porta tem de girar em todas.

## A sequência

A porta é um ângulo que o plugin controla; cada etapa espera a anterior.

**Descer**

1. A porta abre (0° até uns 70°, cerca de meio segundo), com o som de abrir.
2. Fica aberta enquanto o personagem sai do banco e desce os degraus de costas, a mão no puxador.
3. Com os dois pés no chão ele se vira, empurra a porta e ela fecha, com o som de bater.
4. Só então o controle volta para andar.

**Subir:** o mesmo ao contrário (abre, sobe os degraus, senta, puxa a porta).

**O que dá o aspecto de "física":**

- aceleração suave da porta (começa devagar, acelera, freia no fim);
- a porta só começa a fechar no quadro da animação em que a mão encosta nela;
- um pequeno balanço no fim do fechamento, e o som no instante da batida.

**Cuidado:** se o jogador sair andando ou o caminhão se mexer com a porta aberta, o plugin fecha a porta sozinho.

## Ordem sugerida

1. **Parte 1 primeiro:** o boneco descendo e subindo de verdade em terceira pessoa, com os sons, em todos os
   caminhões, mesmo com a porta fechada.
2. **Investigação da porta, num caminhão só:** o plugin registra no log o que o jogo desenha com o jogador a pé ao
   lado da cabine, para ver se a cabine pode ser identificada com segurança. Se der, o resto é trabalhoso mas
   conhecido; se não der, descobre-se cedo.
3. Com a cabine reconhecida: a caixa da porta, o giro, a sequência acima e o ajuste por caminhão.
