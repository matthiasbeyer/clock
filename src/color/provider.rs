use smart_leds::RGB8;

pub trait Provider {
    fn provide_next(&mut self) -> smart_leds::RGB8;

    fn into_iter(self) -> ProviderIter<Self>
    where
        Self: Sized,
    {
        ProviderIter(self)
    }
}

pub struct ProviderIter<P: Provider>(P);

impl<P> ProviderIter<P>
where
    P: Provider,
{
    pub fn provide_next(&mut self) -> smart_leds::RGB8 {
        self.0.provide_next()
    }
}

impl<P> Iterator for ProviderIter<P>
where
    P: Provider,
{
    type Item = RGB8;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.provide_next())
    }
}
