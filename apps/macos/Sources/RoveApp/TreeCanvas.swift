import AppKit
import SwiftUI

struct TreeCanvas: NSViewRepresentable {
  let figure: TreeFigure
  let style: TreeStyle
  @Bindable var model: TreeExplorer

  func makeNSView(context: Context) -> TreeScrollView {
    let scroll = TreeScrollView()
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    scroll.scrollerStyle = .overlay
    scroll.borderType = .noBorder
    scroll.drawsBackground = true
    scroll.backgroundColor = style.canvas.color
    scroll.allowsMagnification = true
    scroll.minMagnification = 1
    scroll.maxMagnification = 4
    scroll.documentView = TreeDrawing()
    return scroll
  }

  func updateNSView(_ scroll: TreeScrollView, context: Context) {
    guard let canvas = scroll.documentView as? TreeDrawing else { return }
    canvas.figure = figure
    canvas.style = style
    canvas.model = model
    canvas.frame = NSRect(origin: .zero, size: figure.size)
    scroll.backgroundColor = style.canvas.color
    scroll.changedZoom = { model.zoom = $0 }
    if abs(scroll.magnification - model.zoom) > 0.001 {
      scroll.setMagnification(model.zoom, centeredAt: scroll.contentView.bounds.center)
    }
    if canvas.focusRevision != model.focusRevision {
      canvas.focusRevision = model.focusRevision
      canvas.hovered = nil
      if let id = model.selected, let p = figure.point(id) {
        canvas.scrollToVisible(NSRect(x: p.x - 60, y: p.y - 50, width: 220, height: 100))
      }
    }
    canvas.needsDisplay = true
  }
}

final class TreeScrollView: NSScrollView {
  var changedZoom: ((Double) -> Void)?
  override func magnify(with event: NSEvent) {
    super.magnify(with: event)
    changedZoom?(magnification)
  }
}

final class TreeDrawing: NSView {
  var figure: TreeFigure?
  var style = TreeStyle.lab
  weak var model: TreeExplorer?
  var focusRevision = -1
  var hovered: Int? {
    didSet {
      guard hovered != oldValue else { return }
      needsDisplay = true
      if (hovered == nil) != (oldValue == nil) {
        (hovered == nil ? NSCursor.arrow : NSCursor.pointingHand).set()
      }
    }
  }
  private var tracking: NSTrackingArea?
  private var dragStart: NSPoint?
  private var scrollStart: NSPoint?
  private var dragged = false
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }

  override func draw(_ dirtyRect: NSRect) {
    guard let figure else { return }
    let tree = figure.tree
    let highlight = Set(figure.selected.map { tree.descendants(of: $0) } ?? [])
    var lineage = Set<Int>()
    var ancestor = hovered
    while let id = ancestor {
      lineage.insert(id)
      ancestor = tree.nodes[id].parent
    }
    func stroke(_ points: [CGPoint], color: NSColor, width: Double) {
      let path = NSBezierPath()
      path.move(to: points[0])
      for point in points.dropFirst() { path.line(to: point) }
      path.lineWidth = width
      path.lineCapStyle = .round
      path.lineJoinStyle = .round
      color.setStroke()
      path.stroke()
    }
    func label(_ value: String, at point: CGPoint, color: NSColor, weight: NSFont.Weight) {
      (value as NSString).draw(
        at: point,
        withAttributes: [.font: NSFont.systemFont(ofSize: 12, weight: weight), .foregroundColor: color])
    }
    func support(_ value: String, at point: CGPoint) {
      let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.monospacedDigitSystemFont(ofSize: 10, weight: .medium),
        .foregroundColor: style.support.color,
      ]
      let size = (value as NSString).size(withAttributes: attributes)
      // A halo in the canvas color keeps the value legible where it crosses a branch.
      style.canvas.color.withAlphaComponent(0.88).setFill()
      NSBezierPath(
        roundedRect: NSRect(x: point.x - 3, y: point.y, width: size.width + 6, height: size.height),
        xRadius: 4, yRadius: 4
      ).fill()
      (value as NSString).draw(at: point, withAttributes: attributes)
    }
    for id in figure.layout.order {
      let node = tree.nodes[id]
      let p = figure.point(id)!
      guard let parent = node.parent, let q = figure.point(parent),
        max(p.y, q.y) >= dirtyRect.minY && min(p.y, q.y) <= dirtyRect.maxY
      else { continue }
      let emphasized = highlight.contains(id) || lineage.contains(id)
      stroke(
        [q, CGPoint(x: q.x, y: p.y), p],
        color: highlight.contains(id)
          ? style.highlight.color : lineage.contains(id) ? style.hover.color : style.branch.color,
        width: emphasized ? style.branchWidth + 0.6 : style.branchWidth)
    }
    for id in figure.layout.order {
      let node = tree.nodes[id]
      let p = figure.point(id)!
      guard p.y >= dirtyRect.minY - 24, p.y <= dirtyRect.maxY + 24 else { continue }
      let selected = id == figure.selected
      let hovering = id == hovered
      if figure.collapsed.contains(id), node.children != nil {
        let wedge = NSBezierPath()
        wedge.move(to: p)
        wedge.line(to: CGPoint(x: figure.tipX, y: p.y - 8))
        wedge.line(to: CGPoint(x: figure.tipX, y: p.y + 8))
        wedge.close()
        wedge.lineJoinStyle = .round
        style.highlight.color.withAlphaComponent(0.18).setFill()
        wedge.fill()
        style.highlight.color.setStroke()
        wedge.stroke()
      }
      if selected || hovering {
        let tone = selected ? style.highlight.color : style.hover.color
        let ring = NSBezierPath(ovalIn: NSRect(x: p.x - 9, y: p.y - 9, width: 18, height: 18))
        if selected {
          tone.withAlphaComponent(0.16).setFill()
          ring.fill()
        }
        tone.withAlphaComponent(selected ? 0.7 : 0.55).setStroke()
        ring.lineWidth = 1
        ring.stroke()
      }
      if node.constrained {
        let diamond = NSBezierPath()
        diamond.move(to: CGPoint(x: p.x, y: p.y - 5))
        diamond.line(to: CGPoint(x: p.x + 5, y: p.y))
        diamond.line(to: CGPoint(x: p.x, y: p.y + 5))
        diamond.line(to: CGPoint(x: p.x - 5, y: p.y))
        diamond.close()
        style.constrained.color.setFill()
        diamond.fill()
      } else {
        style.node.color.setFill()
        let r = selected ? 4.0 : 2.5
        NSBezierPath(ovalIn: NSRect(x: p.x - r, y: p.y - r, width: 2 * r, height: 2 * r)).fill()
      }
      if node.taxon != nil || figure.collapsed.contains(id) {
        label(
          figure.visibleLabel(id), at: figure.labelPoint(id),
          color: hovering ? style.hover.color : style.ink.color,
          weight: selected || hovering ? .semibold : .regular)
      }
      if let value = figure.support(id) { support(value, at: figure.supportPoint(id)) }
    }
    let y = Double(figure.size.height) - 35
    if figure.scale.value > 0 {
      stroke(
        [
          CGPoint(x: 32, y: y - 4), CGPoint(x: 32, y: y), CGPoint(x: 32 + figure.scale.width, y: y),
          CGPoint(x: 32 + figure.scale.width, y: y - 4),
        ], color: style.scale.color, width: 1.5)
    }
    (figure.scaleLabel as NSString).draw(
      at: CGPoint(x: 32, y: y + 6),
      withAttributes: [
        .font: NSFont.monospacedDigitSystemFont(ofSize: 10, weight: .medium),
        .foregroundColor: style.scale.color,
      ])
  }

  override func updateTrackingAreas() {
    super.updateTrackingAreas()
    if let tracking { removeTrackingArea(tracking) }
    let area = NSTrackingArea(
      rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeInKeyWindow, .inVisibleRect],
      owner: self, userInfo: nil)
    addTrackingArea(area)
    tracking = area
  }

  /// The node or label nearest to `location`, if any lies within reach.
  private func hit(_ location: NSPoint) -> Int? {
    guard let figure else { return nil }
    let candidates = figure.layout.order.filter { id in
      let p = figure.point(id)!
      if abs(p.y - location.y) < 9 && abs(p.x - location.x) < 9 { return true }
      if figure.tree.nodes[id].taxon != nil || figure.collapsed.contains(id) {
        let q = figure.labelPoint(id)
        return NSRect(x: q.x, y: q.y - 2, width: figure.labelWidth, height: 20).contains(location)
      }
      return false
    }
    return candidates.min(by: {
      abs(figure.point($0)!.x - location.x) < abs(figure.point($1)!.x - location.x)
    })
  }

  override func mouseMoved(with event: NSEvent) {
    hovered = hit(convert(event.locationInWindow, from: nil))
  }
  override func mouseExited(with event: NSEvent) {
    hovered = nil
  }
  override func mouseDown(with event: NSEvent) {
    window?.makeFirstResponder(self)
    dragStart = event.locationInWindow
    scrollStart = enclosingScrollView?.contentView.bounds.origin
    dragged = false
  }
  override func mouseDragged(with event: NSEvent) {
    guard let start = dragStart, let origin = scrollStart, let scroll = enclosingScrollView else {
      return
    }
    let delta = NSPoint(
      x: event.locationInWindow.x - start.x, y: event.locationInWindow.y - start.y)
    guard abs(delta.x) + abs(delta.y) > 3 else { return }
    dragged = true
    let proposed = NSRect(
      origin: NSPoint(
        x: origin.x - delta.x / scroll.magnification,
        y: origin.y + delta.y / scroll.magnification), size: scroll.contentView.bounds.size)
    scroll.contentView.scroll(to: scroll.contentView.constrainBoundsRect(proposed).origin)
    scroll.reflectScrolledClipView(scroll.contentView)
  }
  override func mouseUp(with event: NSEvent) {
    guard !dragged, let id = hit(convert(event.locationInWindow, from: nil)) else { return }
    model?.selected = id
  }
  override func keyDown(with event: NSEvent) {
    guard let model, let tree = figure?.tree,
      event.modifierFlags.intersection([.command, .control, .option]).isEmpty
    else {
      super.keyDown(with: event)
      return
    }
    switch event.keyCode {
    case 125: model.moveSelection(1)
    case 126: model.moveSelection(-1)
    case 123:
      if let id = model.selected, let parent = tree.nodes[id].parent { model.reveal(parent) }
    case 124:
      if let id = model.selected, let children = tree.nodes[id].children {
        model.reveal(model.rotated.contains(id) ? children[1] : children[0])
      }
    case 49: model.toggleCollapse()
    default: super.keyDown(with: event)
    }
  }
  override func isAccessibilityElement() -> Bool { true }
  override func accessibilityRole() -> NSAccessibility.Role? { .group }
  override func accessibilityLabel() -> String? { figure?.strings("Phylogenetic tree") }
  override func accessibilityHelp() -> String? {
    figure?.strings(
      "Use arrow keys to select nodes and Space to collapse a clade. Node details are available in the inspector."
    )
  }
  override func accessibilityValue() -> Any? { model?.selected.flatMap { figure?.label($0) } }
}

extension CGRect {
  fileprivate var center: CGPoint { CGPoint(x: midX, y: midY) }
}
