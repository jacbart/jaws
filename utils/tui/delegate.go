package tui

import (
	"strconv"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

const (
	selectedSymbol   = "⦿"
	deselectedSymbol = "○"
)

var (
	selectedCount = 0
)

func newItemDelegate(keys *delegateKeyMap) list.DefaultDelegate {
	d := list.NewDefaultDelegate()

	d.UpdateFunc = func(msg tea.Msg, m *list.Model) tea.Cmd {
		var title string
		var selected bool

		if i, ok := m.SelectedItem().(item); ok {
			title = i.Title()
			selected = i.selected
		} else {
			return nil
		}

		switch msg := msg.(type) {
		case tea.KeyMsg:
			switch {
			case key.Matches(msg, keys.choose):
				if selectedCount != 0 {
					return m.NewStatusMessage(statusMessageStyle("You chose " + strconv.Itoa(selectedCount)))
				}
				return m.NewStatusMessage(statusMessageStyle("You chose " + title))
			case key.Matches(msg, keys.remove):
				index := m.Index()
				m.RemoveItem(index)
				if len(m.Items()) == 0 {
					keys.remove.SetEnabled(false)
				}
				return m.NewStatusMessage(statusMessageStyle("Deleted " + title))
			case key.Matches(msg, keys.sel):
				newItem := item{}
				index := m.Index()
				if selected {
					newItem.title = strings.ReplaceAll(title, selectedSymbol, deselectedSymbol)
					newItem.selected = false
					selectedCount--
				} else {
					newItem.title = strings.ReplaceAll(title, deselectedSymbol, selectedSymbol)
					newItem.selected = true
					selectedCount++
					m.CursorDown()
				}
				return m.SetItem(index, newItem)
			}
		}
		return nil
	}

	help := []key.Binding{keys.choose, keys.remove}

	d.ShortHelpFunc = func() []key.Binding {
		return help
	}

	d.FullHelpFunc = func() [][]key.Binding {
		return [][]key.Binding{help}
	}

	return d
}

func newItemStyles() (s list.DefaultItemStyles) {
	s.NormalTitle = lipgloss.NewStyle()

	s.NormalDesc = lipgloss.NewStyle()

	s.SelectedTitle = lipgloss.NewStyle()

	s.SelectedDesc = lipgloss.NewStyle()

	s.DimmedTitle = lipgloss.NewStyle()

	s.DimmedDesc = lipgloss.NewStyle()

	s.FilterMatch = lipgloss.NewStyle().Underline(true)

	return s
}

type delegateKeyMap struct {
	choose key.Binding
	remove key.Binding
	sel    key.Binding
}

// Additional short help entries. This satisfies the help.KeyMap interface and
// is entirely optional.
func (d delegateKeyMap) ShortHelp() []key.Binding {
	return []key.Binding{
		d.choose,
		d.remove,
		d.sel,
	}
}

// Additional full help entries. This satisfies the help.KeyMap interface and
// is entirely optional.
func (d delegateKeyMap) FullHelp() [][]key.Binding {
	return [][]key.Binding{
		{
			d.choose,
			d.remove,
			d.sel,
		},
	}
}

func newDelegateKeyMap() *delegateKeyMap {
	return &delegateKeyMap{
		choose: key.NewBinding(
			key.WithKeys("enter"),
			key.WithHelp("enter", "choose"),
		),
		remove: key.NewBinding(
			key.WithKeys("x", "backspace"),
			key.WithHelp("x", "delete"),
		),
		sel: key.NewBinding(
			key.WithKeys("tab", " "),
			key.WithHelp("tab", "select"),
		),
	}
}
