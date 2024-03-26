package tui

import (
	"strings"

	tea "github.com/charmbracelet/bubbletea"
)

var choice string

type selectorModel struct {
	cursor   int
	choices  []string
	quitting bool
}

func (m selectorModel) Init() tea.Cmd {
	return nil
}

func (m selectorModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c", "q", "esc":
			m.quitting = true
		case "enter":
			// Send the choice on the channel and exit.
			choice = m.choices[m.cursor]
			m.quitting = true
		case "down", "j":
			m.cursor++
			if m.cursor >= len(m.choices) {
				m.cursor = 0
			}
		case "up", "k":
			m.cursor--
			if m.cursor < 0 {
				m.cursor = len(m.choices) - 1
			}
		}
	}

	if m.quitting {
		return m, tea.Quit
	}

	return m, nil
}

func (m selectorModel) View() string {
	s := strings.Builder{}
	s.WriteString("select one\n\n")

	for i := 0; i < len(m.choices); i++ {
		if m.cursor == i {
			s.WriteString("(•) ")
		} else {
			s.WriteString("( ) ")
		}
		s.WriteString(m.choices[i])
		s.WriteString("\n")
	}
	s.WriteString("\n(press q to quit)\n")

	return s.String()
}

func initialSelectorModel(choices []string) selectorModel {
	return selectorModel{
		choices:  choices,
		cursor:   0,
		quitting: false,
	}
}

func SelectorTUI(choices []string) (string, error) {
	m := initialSelectorModel(choices)

	p := tea.NewProgram(m)

	// Run returns the model as a tea.Model.
	err := p.Start()
	if err != nil {
		return "", err
	}

	return choice, nil
}
