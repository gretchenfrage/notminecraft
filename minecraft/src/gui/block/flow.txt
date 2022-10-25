

1. Start with a &'a mut GuiStateFrame
    - at this stage, we know the root-level size and scale
2. Convert to a tree of GuiBlock<'a>
    - (GuiBlock<'a>, size constraints) => (SizedGuiBlock<'a>, size)
    - at various stretches of this recursive tree, size may be determined on
      wind-up or wind-down
        - some GuiBlock types may just pass this through, be agnostic about
          it
        - at the root level, though, size is determined on wind-up
    - positioning cannot be known at this sizing phase though, because a node's
      parent may consider the node's size and/or its siblings' sizes when
      calculating its position 
3. Convert to a tree of SizedGuiBlock<'a>
    - (SizedGuiBlock<'a>) => [(GuiNode<'a>, modifiers (incl. positioning)]
    - we actually do this with a modifier stack visitor pattern that is
      type-parameterized over target (GuiVisitorTarget and GuiVisitor)
    - the tree is flattened
4. Call the relevant callback in each GuiNode with the modifiers
    - we could squash these steps together but we feel it makes the API nicer to
      do it like this
