use prototty_render::*;

#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct TransformRgb24View<V> {
    pub view: V,
}

impl<V> TransformRgb24View<V> {
    pub fn new(view: V) -> Self {
        Self { view }
    }
}

#[cfg_attr(feature = "serialize", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy)]
pub struct TransformRgb24Data<S, T> {
    pub transform_rgb24: S,
    pub data: T,
}

impl<'a, T, V: View<&'a T>, S: ViewTransformRgb24> View<&'a TransformRgb24Data<S, T>>
    for TransformRgb24View<V>
{
    fn view<G: ViewGrid, R: ViewTransformRgb24>(
        &mut self,
        &TransformRgb24Data {
            transform_rgb24,
            ref data,
        }: &'a TransformRgb24Data<S, T>,
        context: ViewContext<R>,
        grid: &mut G,
    ) {
        self.view(
            TransformRgb24Data {
                transform_rgb24,
                data,
            },
            context,
            grid,
        )
    }
    fn visible_bounds<R: ViewTransformRgb24>(
        &mut self,
        &TransformRgb24Data {
            transform_rgb24,
            ref data,
        }: &'a TransformRgb24Data<S, T>,
        context: ViewContext<R>,
    ) -> Size {
        self.visible_bounds(
            TransformRgb24Data {
                transform_rgb24,
                data,
            },
            context,
        )
    }
}

impl<T, V: View<T>, S: ViewTransformRgb24> View<TransformRgb24Data<S, T>>
    for TransformRgb24View<V>
{
    fn view<G: ViewGrid, R: ViewTransformRgb24>(
        &mut self,
        TransformRgb24Data {
            transform_rgb24,
            data,
        }: TransformRgb24Data<S, T>,
        context: ViewContext<R>,
        grid: &mut G,
    ) {
        self.view
            .view(data, context.compose_transform_rgb24(transform_rgb24), grid);
    }
    fn visible_bounds<R: ViewTransformRgb24>(
        &mut self,
        TransformRgb24Data {
            transform_rgb24: _,
            data,
        }: TransformRgb24Data<S, T>,
        context: ViewContext<R>,
    ) -> Size {
        self.view.visible_bounds(data, context)
    }
}