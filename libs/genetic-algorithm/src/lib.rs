use rand::seq::SliceRandom;
use rand::{Rng, RngCore};
use std::ops::Index;

pub trait Individual {
    fn create(chromosome: Chromosome) -> Self;
    fn fitness(&self) -> f32;
    fn chromosome(&self) -> &Chromosome;
}

pub struct GeneticAlgorithm<S> {
    selection_method: S,
    crossover_method: Box<dyn CrossoverMethod>,
    mutation_method: Box<dyn MutationMethod>,
}

impl<S> GeneticAlgorithm<S>
where
    S: SelectionMethod,
{
    pub fn new(
        selection_method: S,
        crossover_method: impl CrossoverMethod + 'static,
        mutation_method: impl MutationMethod + 'static,
    ) -> Self {
        Self {
            selection_method,
            crossover_method: Box::new(crossover_method),
            mutation_method: Box::new(mutation_method),
        }
    }

    pub fn evolve<I>(&self, rng: &mut dyn RngCore, population: &[I]) -> (Vec<I>, Statistics)
    where
        I: Individual,
    {
        assert!(!population.is_empty());

        let new_population = (0..population.len())
            .map(|_| {
                let parent_a = self.selection_method.select(rng, population).chromosome();
                let parent_b = self.selection_method.select(rng, population).chromosome();

                let mut child = self.crossover_method.crossover(rng, parent_a, parent_b);

                self.mutation_method.mutate(rng, &mut child);

                I::create(child)
            })
            .collect();

        let stats = Statistics::new(population);

        (new_population, stats)
    }
}

#[derive(Clone, Debug)]
pub struct Statistics {
    pub min_fitness: f32,
    pub max_fitness: f32,
    pub avg_fitness: f32,
}

impl Statistics {
    fn new<I>(population: &[I]) -> Self where I: Individual, {
        assert!(!population.is_empty());

        let mut min_fitness = population[0].fitness();
        let mut max_fitness = min_fitness;
        let mut sum_fitness = 0.0;

        for individual in population {
            let fitness = individual.fitness();

            min_fitness = min_fitness.min(fitness);
            max_fitness = max_fitness.max(fitness);
            sum_fitness += fitness;
        }

        Self {
            min_fitness,
            max_fitness,
            avg_fitness: sum_fitness / (population.len() as f32),
        }
    }
}

pub trait SelectionMethod {
    fn select<'a, I>(&self, rng: &mut dyn RngCore, population: &'a [I]) -> &'a I
    where
        I: Individual;
}

pub struct RouletteWheelSelection;

impl SelectionMethod for RouletteWheelSelection {
    fn select<'a, I>(&self, rng: &mut dyn RngCore, population: &'a [I]) -> &'a I
    where
        I: Individual,
    {
        population
            .choose_weighted(rng, |individual| individual.fitness())
            .expect("got an empty population")
    }
}

pub struct GaussianMutation {
    chance: f32,
    coeff: f32,
}

impl GaussianMutation {
    pub fn new(chance: f32, coeff: f32) -> Self {
        assert!(chance >= 0.0 && chance <= 1.0);

        Self { chance, coeff }
    }
}

impl MutationMethod for GaussianMutation {
    fn mutate(&self, rng: &mut dyn RngCore, child: &mut Chromosome) {
        for gene in child.iter_mut() {
            let sign = if rng.gen_bool(0.5) { -1.0 } else { 1.0 };

            if rng.gen_bool(self.chance as f64) {
                *gene += sign * self.coeff * rng.gen::<f32>();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chromosome {
    genes: Vec<f32>,
}

pub trait MutationMethod {
    fn mutate(&self, rng: &mut dyn RngCore, child: &mut Chromosome);
}

impl Chromosome {
    pub fn len(&self) -> usize {
        self.genes.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &f32> {
        self.genes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut f32> {
        self.genes.iter_mut()
    }
}

impl Index<usize> for Chromosome {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.genes[index]
    }
}

impl FromIterator<f32> for Chromosome {
    fn from_iter<I: IntoIterator<Item = f32>>(iter: I) -> Self {
        Self {
            genes: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Chromosome {
    type Item = f32;
    type IntoIter = std::vec::IntoIter<f32>;

    fn into_iter(self) -> Self::IntoIter {
        self.genes.into_iter()
    }
}

pub trait CrossoverMethod {
    fn crossover(
        &self,
        rng: &mut dyn RngCore,
        parent_a: &Chromosome,
        parent_b: &Chromosome,
    ) -> Chromosome;
}

#[derive(Clone, Debug)]
pub struct UniformCrossover;

impl CrossoverMethod for UniformCrossover {
    fn crossover(
        &self,
        rng: &mut dyn RngCore,
        parent_a: &Chromosome,
        parent_b: &Chromosome,
    ) -> Chromosome {
        assert_eq!(parent_a.len(), parent_b.len());

        parent_a
            .iter()
            .zip(parent_b.iter())
            .map(|(&a, &b)| if rng.gen_bool(0.5) { a } else { b })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    #[test]
    fn genetic_algorithm() {
        fn individual(genes: &[f32]) -> TestIndividual {
            TestIndividual::create(genes.iter().cloned().collect())
        }

        let mut rng = ChaCha8Rng::from_seed(Default::default());

        let ga = GeneticAlgorithm::new(
            RouletteWheelSelection,
            UniformCrossover,
            GaussianMutation::new(0.5, 0.5),
        );

        let mut population = vec![
            individual(&[1.0, 0.0, 0.0]),
            individual(&[1.0, 1.0, 1.0]),
            individual(&[1.0, 2.0, 1.0]),
            individual(&[1.0, 2.0, 4.0]),
        ];

        for _ in 0..10 {
            population = ga.evolve(&mut rng, &population);
        }

        let expected_population = vec![
            individual(&[0.3157797, 1.7537696, 1.3058136]),
            individual(&[0.9006109, 1.7586328, 2.5140536]),
            individual(&[0.92985255, 1.9546726, 2.4734416]),
            individual(&[0.6470211, 2.1508126, 1.0247333]),
        ];

        assert_eq!(population, expected_population);
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum TestIndividual {
        WithChromosome { chromosome: Chromosome },
        WithFitness { fitness: f32 },
    }

    impl TestIndividual {
        fn new(fitness: f32) -> Self {
            Self::WithFitness { fitness }
        }
    }

    impl PartialEq for Chromosome {
        fn eq(&self, other: &Self) -> bool {
            approx::relative_eq!(self.genes.as_slice(), other.genes.as_slice())
        }
    }

    impl Individual for TestIndividual {
        fn create(chromosome: Chromosome) -> Self {
            Self::WithChromosome { chromosome }
        }

        fn fitness(&self) -> f32 {
            match self {
                Self::WithChromosome { chromosome } => chromosome.iter().sum(),

                Self::WithFitness { fitness } => *fitness,
            }
        }

        fn chromosome(&self) -> &Chromosome {
            match self {
                Self::WithChromosome { chromosome } => &chromosome,
                Self::WithFitness { .. } => {
                    panic!("not supported for TestIndividual::WithFitness")
                }
            }
        }
    }

    #[test]
    fn roulette_wheel_selection() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());

        let population = vec![
            TestIndividual::new(2.0),
            TestIndividual::new(1.0),
            TestIndividual::new(4.0),
            TestIndividual::new(3.0),
        ];

        let mut actual_histogram = BTreeMap::new();

        for _ in 0..1000 {
            let fitness = RouletteWheelSelection
                .select(&mut rng, &population)
                .fitness() as i32;

            *actual_histogram.entry(fitness).or_insert(0) += 1;
        }

        let expected_histogram = BTreeMap::from_iter([(1, 98), (2, 202), (3, 278), (4, 422)]);

        assert_eq!(actual_histogram, expected_histogram);
    }

    #[test]
    fn uniform_crossover() {
        let mut rng = ChaCha8Rng::from_seed(Default::default());

        let parent_a: Chromosome = (1..=100).map(|n| n as f32).collect();
        let parent_b: Chromosome = (1..=100).map(|n| -n as f32).collect();

        let child = UniformCrossover.crossover(&mut rng, &parent_a, &parent_b);

        let diff_a = child.iter().zip(parent_a).filter(|(c, p)| *c != p).count();
        let diff_b = child.iter().zip(parent_b).filter(|(c, p)| *c != p).count();

        assert_eq!(diff_a, 49);
        assert_eq!(diff_b, 51);
    }

    mod gaussian_mutation {
        use super::*;

        fn actual(chance: f32, coeff: f32) -> Vec<f32> {
            let mut rng = ChaCha8Rng::from_seed(Default::default());
            let mut child = vec![1.0, 2.0, 3.0, 4.0, 5.0].into_iter().collect();

            GaussianMutation::new(chance, coeff).mutate(&mut rng, &mut child);

            child.into_iter().collect()
        }

        mod given_zero_chance {
            use approx::assert_relative_eq;

            fn actual(coeff: f32) -> Vec<f32> {
                super::actual(0.0, coeff)
            }

            mod and_zero_coefficient {
                use super::*;

                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(0.0);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }

            mod and_nonzero_coefficient {
                use super::*;

                #[test]
                fn does_not_change_the_original_chromosone() {
                    let actual = actual(0.0);
                    let expected = vec![1.0, 2.0, 3.0, 4.0, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }

        mod given_fifty_fifty_chance {
            use approx::assert_relative_eq;

            fn actual(coeff: f32) -> Vec<f32> {
                super::actual(0.5, coeff)
            }

            mod and_zero_coefficient {
                use super::*;

                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(0.5);
                    let expected = vec![1.0, 1.7756249, 3.0, 4.1596804, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }

            mod and_nonzero_coefficient {
                use super::*;

                #[test]
                fn slightly_changes_the_original_chromosome() {
                    let actual = actual(0.5);
                    let expected = vec![1.0, 1.7756249, 3.0, 4.1596804, 5.0];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }

        mod given_max_chance {
            use approx::assert_relative_eq;

            fn actual(coeff: f32) -> Vec<f32> {
                super::actual(1.0, coeff)
            }

            mod and_zero_coefficient {
                use super::*;

                #[test]
                fn does_not_change_the_original_chromosome() {
                    let actual = actual(1.0);
                    let expected = vec![1.9090631, 2.2324157, 2.5512497, 3.901025, 4.2773824];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }

            mod and_nonzero_coefficient {
                use super::*;

                #[test]
                fn entirely_changes_the_original_chromosome() {
                    let actual = actual(1.0);
                    let expected = vec![1.9090631, 2.2324157, 2.5512497, 3.901025, 4.2773824];

                    assert_relative_eq!(actual.as_slice(), expected.as_slice());
                }
            }
        }
    }
}
