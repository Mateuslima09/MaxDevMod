# Notificações do jogo no celular — relatório de teste (para o dono)

O que foi instalado no PC em 2026-11-20 e ainda **não foi testado dentro do jogo**. Testar com o **app MaxDevMod
aberto** e o **jogo reiniciado** (a ponte mudou para a versão 49: um jogo que já estava aberto continua com o
plugin antigo e o app mostra "jogo não visto" até reiniciar). Marque cada item e anote o que achou. Nada foi para
o GitHub.

## Como funciona (resumo)

- O mesmo sistema das notificações do TikTok: o banner aparece no topo do celular, com som. Celular guardado, só
  o **topo do celular sobe** no canto, mostra a notificação e desce sozinho.
- Agora, além do TikTok, aparecem notificações do **jogo**, cada uma com o ícone e o nome do app:
  - **Nupay**: lembrete de pagar o frentista depois de abastecer a pé, pagamento de frete recebido, multa (com a infração), pedágio, balsa, trem, combustível pago no
    frentista, multa por cancelar frete, e a diferença do save (oficina, compras, diesel do posto normal:
    "Saída da conta").
  - **Caminhão**: combustível na reserva (o aviso do próprio painel), batida, dano passando de 10%, 25%, 50% e
    75%, caminhão consertado, hora de descansar (60 min antes) e "você está cansado".
  - **Fretes**: novo frete aceito (carga, destino e valor), faltam 60 min para o prazo, entrega atrasada.
- O som muda: subindo para dinheiro entrando, dois toques graves para aviso (multa, dano, reserva, atraso).
- Cada grupo liga e desliga em **Ajustes › Notificações do jogo**. Notificações repetidas não ficam spamando
  (vários pedágios seguidos viram um só "×2", a reserva não avisa de novo por 10 min, cada degrau de dano avisa
  uma vez, a batida no máximo a cada 20 s).
- O que já estava assim quando o jogo abriu (caminhão já danificado, frete já aceito, já cansado) **não** avisa.

## 1. Ajustes

- [ ] Celular › Ajustes mostra a lista **Notificações do jogo** com 7 linhas, cada uma com Ligado/Desligado:
      Pagamentos recebidos, Pagamentos feitos, Multas, Combustível (reserva e pagar frentista), Dano no caminhão, Descanso,
      Fretes e prazos.
- [ ] **Testar notificações do jogo** mostra uma de cada (7 banners, um depois do outro), com o ícone certo
      (Nupay roxo, Caminhão azul, Fretes laranja) e o som.
- [ ] Desligar um grupo (ex.: Multas) e testar de novo: a de multa **não aparece**. Ligar de volta: aparece.
- [ ] Com o celular **guardado**, numa notificação real (ex.: um pedágio): o topo do celular sobe, mostra e desce.
- [ ] Depois de assinar a nota fiscal ou pagar o frentista, o celular **não** repete a última notificação
      (antes ele mostrava de novo a última ao voltar para a tela normal).
- [ ] Fechar e abrir o MaxDevMod: o que você desligou continua desligado.

## 2. Dinheiro (Nupay)

- [ ] Entregar um frete: "Pix recebido: R$ … pelo frete · Carga → Cidade" (na moeda escolhida no Nupay).
- [ ] Passar num pedágio: "Pedágio pago: …".
- [ ] Levar uma multa (radar, sinal vermelho, batida): "Você levou uma multa de … · Radar".
- [ ] Balsa ou trem: "Balsa paga: … · Origem → Destino".
- [ ] Abastecer pelo frentista (abastecimento manual) e pagar no Nupay: "Combustível pago: …".
- [ ] Ir na oficina ou comprar algo e **salvar** o jogo (ou esperar o autosave): "Saída da conta: …".
      Ao abrir o jogo, a primeira leitura do save **não** deve avisar nada.

- [ ] Abastecer a pé e soltar: "Pague R$ X ao frentista…" (Nupay). Desligando "Combustível" nos Ajustes, não aparece.

## 3. Caminhão

- [ ] Rodar até acender a luz da reserva no painel: "Combustível na reserva: dá uns X km".
- [ ] Bater em algo dirigindo: "Batida! O caminhão está com X% de dano (cabine)".
- [ ] Batidas seguidas não viram uma enxurrada (no máximo uma a cada 20 s).
- [ ] Dano passando de 10% / 25% / 50% / 75%: "Caminhão com X% de dano (motor). Passe na oficina."
- [ ] Consertar na oficina (parado): "Caminhão consertado ✔".
- [ ] Trocar de caminhão na garagem: **não** deve avisar batida nem dano.
- [ ] Com o cansaço ligado no jogo: 60 min de jogo antes da hora de dormir aparece "Hora de descansar em X min";
      na hora, "Você está cansado: pare para dormir". Depois de dormir, volta a avisar na próxima vez.

## 4. Fretes

- [ ] Aceitar um frete: "Novo frete · Carga → Cidade · R$ …".
- [ ] Faltando 60 min (do relógio do jogo) para o prazo: "Faltam X min para o prazo da entrega".
- [ ] Passou do prazo: "A entrega está atrasada" (uma vez só).
- [ ] Frete especial / sem prazo: não avisa prazo.

## Anote

- Alguma notificação que apareceu na hora errada, repetida demais, ou que faltou.
- Se o texto ficou grande demais no banner (corta em 2 linhas).
- Outras notificações que você quer (ex.: velocidade, pneu, AdBlue, chegada no destino).
