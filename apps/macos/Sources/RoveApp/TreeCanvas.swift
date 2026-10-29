import AppKit
import SwiftUI

struct TreeCanvas: NSViewRepresentable {
  let figure: TreeFigure
  @Bindable var model: TreeExplorer

  func makeNSView(context: Context) -> TreeScrollView {
    let scroll = TreeScrollView()
    scroll.hasVerticalScroller = true
    scroll.hasHorizontalScroller = true
    scroll.autohidesScrollers = true
    scroll.scrollerStyle = .overlay
    scroll.borderType = .noBorder
    scroll.drawsBackground = true
    scroll.backgroundColor = .init(srgbRed: 248 / 255, green: 247 / 255, blue: 252 / 255, alpha: 1)
    scroll.allowsMagnification = true
    scroll.minMagnification = 1
    scroll.maxMagnification = 4
    scroll.documentView = TreeDrawing()
    return scroll
  }

  func updateNSView(_ scroll: TreeScrollView, context: Context) {
    guard let canvas = scroll.documentView as? TreeDrawing else { return }
    canvas.figure = figure
    canvas.model = model
    canvas.frame = NSRect(origin: .zero, size: figure.size)
    scroll.changedZoom = { model.zoom = $0 }
    if abs(scroll.magnification - model.zoom) > 0.001 {
      scroll.setMagnification(model.zoom, centeredAt: scroll.contentView.bounds.center)
    }
    if canvas.focusRevision != model.focusRevision {
      canvas.focusRevision = model.focusRevision
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
  weak var model: TreeExplorer?
  var focusRevision = -1
  private var dragStart: NSPoint?
  private var scrollStart: NSPoint?
  private var dragged = false
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }

  override func draw(_ dirtyRect: NSRect) {
    guard let figure else { return }
    let textColor = NSColor(srgbRed: 48 / 255, green: 49 / 255, blue: 65 / 255, alpha: 1)
    let accent = NSColor(srgbRed: 128 / 255, green: 100 / 255, blue: 193 / 255, alpha: 1)
    let imposed = NSColor(srgbRed: 182 / 255, green: 92 / 255, blue: 124 / 255, alpha: 1)
    let highlight = Set(figure.selected.map { figure.tree.descendants(of: $0) } ?? [])
    func stroke(_ a: CGPoint, _ b: CGPoint, _ c: CGPoint, color: NSColor) {
      let path = NSBezierPath()
      path.move(to: a)
      path.line(to: b)
      path.line(to: c)
      path.lineWidth = 1.3
      color.setStroke()
      path.stroke()
    }
    func text(_ value: String, at point: CGPoint, size: Double = 12) {
      (value as NSString).draw(
        at: point,
        withAttributes: [.font: NSFont.systemFont(ofSize: size), .foregroundColor: textColor])
    }
    for id in figure.layout.order {
      let node = figure.tree.nodes[id]
      let p = figure.point(id)!
      if let parent = node.parent, let q = figure.point(parent),
        max(p.y, q.y) >= dirtyRect.minY && min(p.y, q.y) <= dirtyRect.maxY
      {
        stroke(
          q, CGPoint(x: q.x, y: p.y), p,
          color: highlight.contains(id)
            ? accent : .init(srgbRed: 117 / 255, green: 121 / 255, blue: 141 / 255, alpha: 1))
      }
    }
    for id in figure.layout.order {
      let node = figure.tree.nodes[id]
      let p = figure.point(id)!
      guard p.y >= dirtyRect.minY - 24, p.y <= dirtyRect.maxY + 24 else { continue }
      if figure.collapsed.contains(id), node.children != nil {
        let path = NSBezierPath()
        path.move(to: p)
        path.line(to: CGPoint(x: figure.tipX, y: p.y - 8))
        path.line(to: CGPoint(x: figure.tipX, y: p.y + 8))
        path.close()
        accent.withAlphaComponent(0.18).setFill()
        path.fill()
        accent.setStroke()
        path.stroke()
      }
      if id == figure.selected {
        accent.withAlphaComponent(0.16).setFill()
        NSBezierPath(ovalIn: NSRect(x: p.x - 9, y: p.y - 9, width: 18, height: 18)).fill()
      }
      if node.constrained {
        let path = NSBezierPath()
        path.move(to: CGPoint(x: p.x, y: p.y - 5))
        path.line(to: CGPoint(x: p.x + 5, y: p.y))
        path.line(to: CGPoint(x: p.x, y: p.y + 5))
        path.line(to: CGPoint(x: p.x - 5, y: p.y))
        path.close()
        imposed.setFill()
        path.fill()
      } else {
        accent.setFill()
        let r = id == figure.selected ? 4.0 : 2.5
        NSBezierPath(ovalIn: NSRect(x: p.x - r, y: p.y - r, width: 2 * r, height: 2 * r)).fill()
      }
      if node.taxon != nil || figure.collapsed.contains(id) {
        text(figure.visibleLabel(id), at: figure.labelPoint(id))
      }
      if let support = figure.support(id) { text(support, at: figure.supportPoint(id), size: 10) }
    }
    let y = Double(figure.size.height) - 35
    if figure.scale.value > 0 {
      stroke(
        CGPoint(x: 32, y: y - 3), CGPoint(x: 32, y: y), CGPoint(x: 32 + figure.scale.width, y: y),
        color: textColor)
    }
    text(figure.scaleLabel, at: CGPoint(x: 32, y: y + 5), size: 10)
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
    guard !dragged, let figure else { return }
    let location = convert(event.locationInWindow, from: nil)
    let candidates = figure.layout.order.filter { id in
      let p = figure.point(id)!
      if abs(p.y - location.y) < 9 && abs(p.x - location.x) < 9 { return true }
      if figure.tree.nodes[id].taxon != nil || figure.collapsed.contains(id) {
        let q = figure.labelPoint(id)
        return NSRect(x: q.x, y: q.y - 2, width: figure.labelWidth, height: 20).contains(location)
      }
      return false
    }
    if let id = candidates.min(by: {
      abs(figure.point($0)!.x - location.x) < abs(figure.point($1)!.x - location.x)
    }) {
      model?.selected = id
    }
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
