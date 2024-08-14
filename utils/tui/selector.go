package tui

import (
	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/jacbart/jaws/utils/style"
)

type listKeyMap struct {
	Quit key.Binding
}

func newListKeyMap() *listKeyMap {
	return &listKeyMap{
		Quit: key.NewBinding(
			key.WithKeys("ctrl+c", "q", "esc"),
			key.WithHelp("q", "quit"),
		),
	}
}

var (
	appStyle   = lipgloss.NewStyle().Padding(1, 2)
	titleStyle = lipgloss.NewStyle().
			Foreground(style.Blue).
			Background(style.White).
			Padding(0, 1)

	statusMessageStyle = lipgloss.NewStyle().
				Foreground(lipgloss.AdaptiveColor{Light: "#767676", Dark: "#fefefe"}).
				Render
)

type item struct {
	title       string
	description string
	selected    bool
}

func (i item) Title() string       { return i.title }
func (i item) Description() string { return i.description }
func (i item) FilterValue() string { return i.title }

type fuzzySelectorModel struct {
	list         list.Model
	keys         *listKeyMap
	delegateKeys *delegateKeyMap
	choiceList   []string
	quitting     bool
}

func (m fuzzySelectorModel) Init() tea.Cmd {
	return tea.EnterAltScreen
}

func (m fuzzySelectorModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		h, v := appStyle.GetFrameSize()
		m.list.SetSize(msg.Width-h, msg.Height-v)
	case tea.KeyMsg:
		if m.list.FilterState() == list.Filtering {
			break
		}
		switch {
		case key.Matches(msg, m.keys.Quit):
			m.quitting = true
		}
	}

	if m.quitting {
		return m, tea.Quit
	}
	newListModel, cmd := m.list.Update(msg)
	m.list = newListModel
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

func (m fuzzySelectorModel) View() string {
	return appStyle.Render(m.list.View())
}

func newFuzSelModel(choiceList []string) fuzzySelectorModel {
	var (
		delegateKeys = newDelegateKeyMap()
		listKeys     = newListKeyMap()
	)

	// create item list of selector
	l := len(choiceList)
	items := make([]list.Item, l)
	for i := 0; i < l; i++ {
		c := item{
			title:       "○ " + choiceList[i],
			description: "",
			selected:    false,
		}
		items[i] = c
	}

	// Setup list
	delegate := newItemDelegate(delegateKeys)
	// itemStyles := newItemStyles()
	delegate.Styles.NormalTitle = delegate.Styles.NormalTitle.Foreground(style.White)
	delegate.Styles.SelectedTitle = delegate.Styles.SelectedTitle.Foreground(style.Blue).BorderLeftForeground(style.Blue)
	delegate.Styles.NormalDesc.ColorWhitespace(false)
	selList := list.New(items, delegate, 0, 0)
	selList.Title = "jaws"
	selList.Styles.Title = titleStyle
	selList.SetShowStatusBar(false)
	selList.AdditionalFullHelpKeys = func() []key.Binding {
		return []key.Binding{
			listKeys.Quit,
		}
	}

	// return the model
	return fuzzySelectorModel{
		list:         selList,
		keys:         listKeys,
		delegateKeys: delegateKeys,
		choiceList:   choiceList,
		quitting:     false,
	}
}

func FuzzySelectorTUI(choiceList []string) (string, error) {
	m := newFuzSelModel(choiceList)

	p := tea.NewProgram(m)

	// Run returns the model as a tea.Model.
	err := p.Start()
	if err != nil {
		return "", err
	}

	return choice, nil
}
