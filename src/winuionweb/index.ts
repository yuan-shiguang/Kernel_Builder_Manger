/**
 * WinUIonWeb 接入层
 * ------------------------------------------------------------------
 * WinUIonWeb 目前并未发布到 npm，本项目以「源码内置(vendor)」的方式使用：
 *   把官方仓库的 src/components、src/styles、src/assets、src/utils 复制到
 *   src/winuionweb/ 下（保持相对目录结构不变，内部相对引用依旧成立）。
 *
 * 作者在 gallery/main.ts 中通过 app.component() 全局注册了一大批控件，
 * 并刻意使用 XAML 风格的名字（如 `Grid.RowDefinitions`、`Button.Flyout`）。
 * 这里把这套注册逻辑抽出来复用，并额外把 src/components/*.vue 全部注册为
 * 全局组件，页面里就可以直接写 <Button> / <TextBlock> / <NavigationView>。
 */
import type { App } from 'vue'

import './styles/theme.css'

import Grid from './components/Grid.vue'
import StackPanel from './components/StackPanel.vue'
import Border from './components/Border.vue'
import Canvas from './components/Canvas.vue'
import RelativePanel from './components/RelativePanel.vue'
import VariableSizedWrapGrid from './components/VariableSizedWrapGrid.vue'
import Page from './components/Page.vue'
import ColumnDefinition from './components/ColumnDefinition.vue'
import RowDefinition from './components/RowDefinition.vue'
import GridColumnDefinitions from './components/GridColumnDefinitions.vue'
import GridRowDefinitions from './components/GridRowDefinitions.vue'
import Image from './components/Image.vue'
import Rectangle from './components/Rectangle.vue'
import FontIcon from './components/FontIcon.vue'
import SymbolIcon from './components/SymbolIcon.vue'
import Flyout from './components/Flyout.vue'
import ContentDialog from './components/ContentDialog.vue'
import Popup from './components/Popup.vue'
import TeachingTip from './components/TeachingTip.vue'
import ToolTip from './components/ToolTip.vue'
import Expander from './components/Expander.vue'
import {
  ExpanderContent,
  ExpanderDescription,
  ExpanderHeader,
  ExpanderHeaderControls,
  ExpanderHeaderIcon,
} from './components/ExpanderProperties'
import {
  DropDownButtonContent,
  DropDownButtonFlyout,
  MenuFlyout,
  MenuFlyoutItem,
  MenuFlyoutItemIcon,
} from './components/DropDownButtonProperties'
import { ButtonFlyout } from './components/Button.vue'
import { SplitButtonFlyout } from './components/SplitButton.vue'
import { ToggleSplitButtonFlyout } from './components/ToggleSplitButton.vue'
import { ToolTipServiceToolTip } from './components/ToolTipServiceProperties'
import {
  TeachingTipContent,
  TeachingTipHeroContent,
  TeachingTipIconSource,
} from './components/TeachingTipProperties'
import {
  CollectionItemTemplate,
  CollectionItemsPanel,
  CollectionLayout,
  DataTemplate,
  ItemsPanelTemplate,
  StackLayout,
  XamlSetter,
  XamlStyle,
  UniformGridLayout,
} from './components/CollectionProperties'

/** 所有单文件控件（不含 gallery 演示页） */
const modules = import.meta.glob('./components/*.vue', { eager: true }) as Record<
  string,
  { default: any }
>

const pascalName = (file: string) => file.replace('./components/', '').replace('.vue', '')

export function registerWinUI(app: App) {
  // 1. 批量注册 src/components/*.vue（按文件名，即 XAML 类型名）
  for (const [file, mod] of Object.entries(modules)) {
    if (mod?.default) app.component(pascalName(file), mod.default)
  }

  // 2. 布局类控件按 XAML 类型名显式注册（覆盖同名，保证子组件可用）
  app.component('Grid', Grid)
  app.component('StackPanel', StackPanel)
  app.component('Border', Border)
  app.component('Canvas', Canvas)
  app.component('RelativePanel', RelativePanel)
  app.component('VariableSizedWrapGrid', VariableSizedWrapGrid)
  app.component('ItemsWrapGrid', VariableSizedWrapGrid)
  app.component('VirtualizingStackPanel', StackPanel)
  app.component('Page', Page)
  app.component('ColumnDefinition', ColumnDefinition)
  app.component('RowDefinition', RowDefinition)
  app.component('Grid.ColumnDefinitions', GridColumnDefinitions)
  app.component('Grid.RowDefinitions', GridRowDefinitions)
  app.component('Image', Image)
  app.component('Rectangle', Rectangle)

  // 3. 图标 / 浮出层 / 对话框
  app.component('FontIcon', FontIcon)
  app.component('SymbolIcon', SymbolIcon)
  app.component('SymbolIconSource', SymbolIcon)
  app.component('Flyout', Flyout)
  app.component('ContentDialog', ContentDialog)
  app.component('Popup', Popup)
  app.component('TeachingTip', TeachingTip)
  app.component('TeachingTip.Content', TeachingTipContent)
  app.component('TeachingTip.HeroContent', TeachingTipHeroContent)
  app.component('TeachingTip.IconSource', TeachingTipIconSource)
  app.component('ToolTip', ToolTip)
  app.component('ToolTipService.ToolTip', ToolTipServiceToolTip)

  // 4. 复合控件的属性元素
  app.component('Expander', Expander)
  app.component('Expander.Header', ExpanderHeader)
  app.component('Expander.Content', ExpanderContent)
  app.component('Expander.Description', ExpanderDescription)
  app.component('Expander.HeaderIcon', ExpanderHeaderIcon)
  app.component('Expander.HeaderControls', ExpanderHeaderControls)

  app.component('Button.Flyout', ButtonFlyout)
  app.component('SplitButton.Flyout', SplitButtonFlyout)
  app.component('ToggleSplitButton.Flyout', ToggleSplitButtonFlyout)
  app.component('DropDownButton.Flyout', DropDownButtonFlyout)
  app.component('DropDownButton.Content', DropDownButtonContent)
  app.component('MenuFlyout', MenuFlyout)
  app.component('MenuFlyoutItem', MenuFlyoutItem)
  app.component('MenuFlyoutItem.Icon', MenuFlyoutItemIcon)

  // 5. 集合控件模板
  app.component('DataTemplate', DataTemplate)
  app.component('ItemsPanelTemplate', ItemsPanelTemplate)
  app.component('ItemsStackPanel', StackPanel)
  app.component('CollectionLayout', CollectionLayout)
  app.component('StackLayout', StackLayout)
  app.component('UniformGridLayout', UniformGridLayout)
  app.component('ListView.ItemTemplate', CollectionItemTemplate)
  app.component('GridView.ItemTemplate', CollectionItemTemplate)
  app.component('ItemsRepeater.ItemTemplate', CollectionItemTemplate)
  app.component('ItemsView.ItemTemplate', CollectionItemTemplate)
  app.component('TreeView.ItemTemplate', CollectionItemTemplate)
  app.component('FlipView.ItemTemplate', CollectionItemTemplate)
  app.component('ListView.ItemsPanel', CollectionItemsPanel)
  app.component('GridView.ItemsPanel', CollectionItemsPanel)
  app.component('Style', XamlStyle)
  app.component('Setter', XamlSetter)
}

export const WinUIComponents = {
  Grid,
  StackPanel,
  Border,
  Page,
  Image,
  FontIcon,
  SymbolIcon,
  Flyout,
  ContentDialog,
  TeachingTip,
  ToolTip,
  Expander,
}

export default registerWinUI
