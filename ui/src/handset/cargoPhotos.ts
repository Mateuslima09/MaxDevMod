import type { JobMarketOffer } from '../shared/host'

/**
 * Maps any cargo offer (ETS2, ATS, Mapa Minas Gerais, Eldorado PRO, custom BR/EU mods) to a realistic
 * themed cargo photo under `ui/public/cargo/*.jpg` whenever a mod's `.scs` archive is encrypted or lacks
 * a readable `.dds` thumbnail.
 */
export type CargoPhotoCategory =
  | 'steel'
  | 'tanker'
  | 'grains'
  | 'produce'
  | 'crates'
  | 'machinery'
  | 'timber'
  | 'construction'
  | 'food_cold'
  | 'automotive'
  | 'chemicals'
  | 'general'

function normalize(input: string): string {
  return input
    .toLowerCase()
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
}

function hasAny(text: string, words: readonly string[]): boolean {
  return words.some((w) => text.includes(w))
}

export function cargoPhotoCategory(offer: Pick<JobMarketOffer, 'cargoId' | 'cargoName' | 'adrClass' | 'overweight' | 'oversize' | 'fragile'>): CargoPhotoCategory {
  const text = normalize(`${offer.cargoName ?? ''} ${offer.cargoId ?? ''}`)

  // 1. Fuel / Tankers / Empty return tankers ("FRETE DE RETORNO" = combustvaz) / Flammable ADR
  if (
    hasAny(text, [
      'combust',
      'diesel',
      'gasolina',
      'etanol',
      'alcool',
      'petroleo',
      'querosene',
      'oleo',
      'fuel',
      'petrol',
      'oil',
      'glp',
      'gas',
      'lpg',
      'propane',
      'butane',
      'tanque',
      'tanker',
      'frete de retorno',
      'retorno',
      'asfalto',
      'asphalt',
      'betume',
    ])
  ) {
    return 'tanker'
  }

  // 2. Steel / Rebar / Iron / Metal structures
  if (
    hasAny(text, [
      'barra',
      'vergalh',
      'ferro',
      'aco',
      'steel',
      'rebar',
      'iron',
      'metal',
      'viga',
      'tubo',
      'pipe',
      'bobina',
      'coil',
      'chapa',
      'aluminio',
      'cobre',
      'sucata',
      'scrap',
      'perfil',
      'estrutura',
      'andaime',
      'arame',
      'cabo',
    ])
  ) {
    return 'steel'
  }

  // 3. Heavy Machinery / Tractors / Excavators / Oversize industrial equipment
  if (
    hasAny(text, [
      'trator',
      'tractor',
      'escavad',
      'excavat',
      'colheit',
      'harvester',
      'maquina',
      'machine',
      'empilhadeira',
      'forklift',
      'guindaste',
      'crane',
      'gerador',
      'generator',
      'transformador',
      'transformer',
      'turbina',
      'retroescav',
      'pa carregadeira',
      'loader',
      'roller',
      'bulldozer',
      'dozer',
      'caldeira',
      'locomotiv',
      'barco',
      'iate',
      'yacht',
      'helicopter',
    ])
  ) {
    return 'machinery'
  }

  // 4. Timber / Logs / Forestry / Wood / Paper
  if (
    hasAny(text, [
      'madeira',
      'tora',
      'eucalipto',
      'pinus',
      'tabua',
      'compensado',
      'celulose',
      'papel',
      'lenha',
      'carvao',
      'wood',
      'timber',
      'log',
      'lumber',
      'plank',
      'plywood',
      'charcoal',
      'serragem',
      'cavaco',
      'bambu',
      'palha',
    ])
  ) {
    return 'timber'
  }

  // 5. Grains / Agriculture / Bulk harvest / Fertilizers
  if (
    hasAny(text, [
      'milho',
      'soja',
      'trigo',
      'cafe',
      'arroz',
      'feijao',
      'acucar',
      'racao',
      'farinha',
      'adubo',
      'fertiliz',
      'granel',
      'farelo',
      'algodao',
      'aveia',
      'cevada',
      'semente',
      'grao',
      'grain',
      'corn',
      'wheat',
      'soy',
      'coffee',
      'rice',
      'sugar',
      'flour',
      'seed',
      'barley',
      'rye',
      'sal',
      'salt',
      'sorgo',
      'amendoim',
      'calcario',
    ])
  ) {
    return 'grains'
  }

  // 6. Fresh Fruits & Vegetables / Produce
  if (
    hasAny(text, [
      'limao',
      'laranja',
      'maca',
      'banana',
      'tomate',
      'batata',
      'cebola',
      'cenoura',
      'alface',
      'fruta',
      'verdura',
      'legume',
      'hortifruti',
      'uva',
      'manga',
      'melancia',
      'abacaxi',
      'morango',
      'abacate',
      'mamão',
      'mamao',
      'alho',
      'pimentao',
      'mandioca',
      'lemon',
      'lime',
      'orange',
      'apple',
      'tomato',
      'potato',
      'onion',
      'fruit',
      'vegetable',
      'produce',
      'planta',
      'flor',
      'flower',
    ])
  ) {
    return 'produce'
  }

  // 7. Cold Food / Meat / Dairy / Beverages / Supermarket
  if (
    hasAny(text, [
      'carne',
      'frango',
      'peixe',
      'suino',
      'bovino',
      'leite',
      'queijo',
      'iogurte',
      'manteiga',
      'sorvete',
      'cerveja',
      'refrigerante',
      'bebida',
      'agua',
      'suco',
      'vinho',
      'alimento',
      'comida',
      'congelad',
      'frios',
      'embutid',
      'supermercad',
      'mercado',
      'pao',
      'biscoito',
      'chocolate',
      'doce',
      'ovo',
      'meat',
      'chicken',
      'beef',
      'pork',
      'fish',
      'milk',
      'cheese',
      'dairy',
      'ice cream',
      'beer',
      'drink',
      'beverage',
      'water',
      'juice',
      'wine',
      'food',
      'frozen',
      'yogurt',
      'pasta',
      'canned',
      'enlatad',
    ])
  ) {
    return 'food_cold'
  }

  // 8. Construction materials (cement, bricks, tiles, sand, gravel, concrete)
  if (
    hasAny(text, [
      'cimento',
      'tijolo',
      'telha',
      'areia',
      'brita',
      'pedra',
      'cascalho',
      'concreto',
      'bloco',
      'ceramica',
      'piso',
      'azulejo',
      'argamassa',
      'gesso',
      'cal ',
      'marmore',
      'granito',
      'construc',
      'obra',
      'cement',
      'brick',
      'tile',
      'sand',
      'gravel',
      'stone',
      'concrete',
      'block',
      'roof',
      'plaster',
      'marble',
      'mortar',
      'terra',
      'minerio',
      'ore',
    ])
  ) {
    return 'construction'
  }

  // 9. Automotive / Vehicles / Tires / Engines / Parts
  if (
    hasAny(text, [
      'carro',
      'veiculo',
      'caminhao',
      'onibus',
      'moto',
      'pneu',
      'roda',
      'motor',
      'peca',
      'autopeca',
      'bateria',
      'chassi',
      'eixo',
      'transmissao',
      'car',
      'vehicle',
      'truck',
      'van',
      'suv',
      'tire',
      'tyre',
      'wheel',
      'engine',
      'axle',
      'battery',
      'part',
    ])
  ) {
    return 'automotive'
  }

  // 10. Chemicals / Industrial liquids / Pharma / Medical / Glass
  if (
    hasAny(text, [
      'quimic',
      'acido',
      'cloro',
      'soda',
      'tinta',
      'verniz',
      'resina',
      'solvente',
      'veneno',
      'defensivo',
      'agrotoxico',
      'limpeza',
      'detergente',
      'sabao',
      'remedio',
      'medic',
      'vacina',
      'farmac',
      'hospital',
      'vidro',
      'chemical',
      'acid',
      'chlorine',
      'paint',
      'medical',
      'vaccine',
      'pharma',
      'glass',
      'pesticid',
      'explosiv',
      'dynamite',
      'firework',
      'fogos',
    ]) ||
    offer.adrClass > 0
  ) {
    return 'chemicals'
  }

  // 11. Crates / Empty boxes / Pallets / Plastics / Packaging
  if (
    hasAny(text, [
      'caixa',
      'palete',
      'pallet',
      'embalag',
      'plastico',
      'papelao',
      'garrafa',
      'lata',
      'saco',
      'vazia',
      'vazio',
      'crate',
      'box',
      'empty',
      'plastic',
      'cardboard',
      'bottle',
      'packaging',
    ])
  ) {
    return 'crates'
  }

  // 12. Flag-based fallbacks
  if (offer.oversize || offer.overweight) return 'machinery'

  return 'general'
}

export function cargoFallbackPhoto(offer: Pick<JobMarketOffer, 'cargoId' | 'cargoName' | 'adrClass' | 'overweight' | 'oversize' | 'fragile'>): string {
  return `/cargo/${cargoPhotoCategory(offer)}.jpg`
}
